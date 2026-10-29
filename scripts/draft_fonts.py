"""Do the drafts draw text only in the app's own fonts? `python scripts/draft_fonts.py`.

plans/core/gui.md "Prototypes before implementation", the Fonts bullet: a draft draws text only
in the font files the app bundles, loaded by `@font-face`, so that what the maintainer reviews
is what egui draws. A family the browser cannot load, or a glyph none of the files has, falls
back to a system font silently (the drafts named Ubuntu, installed nowhere, and were reviewed in
Segoe UI for days). Two checks:

- the rendered pages: every text node of the host and of each tool's view (the Save editor's
  with its first team selected, which shows the Tactics pitch), through Chromium's
  `CSS.getPlatformFontsForNode`, the font each glyph was really drawn in; a font that is not
  an `@font-face` one is reported with the text it drew;
- the sources: canvas text, which no DOM node holds, so every `font`/`font-family` declaration
  and every canvas `.font =` assignment must name the stylesheet's chains (`var(--font-body)`,
  `var(--font-mono)`, `FONT_BODY`) or the icon font, never a family of its own.

A member's own text is the exception: a character the sample saves' names carry that no bundled
file has (`MEMBER_TEXT`) falls back to a system font in the app as well (eframe's system
fallback, kept for what a member types), so a node whose only such characters are those is no
finding.

Needs Python's `playwright` with its Chromium (`playwright install chromium`) and fontTools; a
development tool, not a gate. Exit 1 on any finding, 0 when the drafts draw only bundled fonts.
"""

import re
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
DRAFTS = ROOT / "docs" / "prototypes"
PAGES = ["", "#save_editor", "#ruleset_editor", "#export_upgrader", "#team_compiler"]
# The sample saves' player and team names (tools/save_editor.js): members' typing, not the
# suite's symbols.
MEMBER_TEXT = set("♂ＡＢＣＩＬＮＯＲＳＴＶ")

# A CSS `font:`/`font-family:` declaration, or a canvas `.font =` assignment.
DECLARATION = re.compile(r"font(?:-family)?\s*:\s*([^;}\n]+)|\.font\s*=\s*([^;\n]+)")
# `--font-label`/`FONT_LABEL`: a bundled bold weight for a label that imitates the game's own
# (the pitch's formation name), the one exception to the drafts' no-bold rule.
ALLOWED = re.compile(r'var\(--font-(?:body|mono|label)\)|FONT_(?:BODY|LABEL)|"Icons"|inherit')


def source_findings():
    findings = []
    for path in [DRAFTS / "index.html", *sorted((DRAFTS / "tools").glob("*.js"))]:
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if line.lstrip().startswith(("//", "/*", "*")) or "@font-face" in line:
                continue
            for match in DECLARATION.finditer(line):
                value = match.group(1) or match.group(2)
                # A declaration of size or weight alone (`font-size`, `font-weight`) does not
                # match `font:`; one naming a family must name an allowed chain.
                if '"' in value or "'" in value or re.search(r"[a-z]-?serif|monospace", value):
                    if not ALLOWED.search(value):
                        findings.append(f"{path.relative_to(ROOT)}:{number}: {value.strip()}")
    return findings


def text_nodes(node, out):
    if node.get("nodeType") == 3 and node.get("nodeValue", "").strip():
        out.append((node["nodeId"], node["nodeValue"]))
    for child in node.get("children", []) + node.get("shadowRoots", []):
        text_nodes(child, out)
    return out


def bundled_codepoints():
    codepoints = set()
    for path in (DRAFTS / "assets" / "fonts").glob("*.ttf"):
        codepoints.update(TTFont(path).getBestCmap())
    return codepoints


def page_findings():
    findings = []
    bundled = bundled_codepoints()
    with sync_playwright() as pw:
        browser = pw.chromium.launch()
        for fragment in PAGES:
            # A tab per page: the host reloads itself on a fragment change.
            page = browser.new_page(viewport={"width": 1360, "height": 1250})
            page.goto((DRAFTS / "index.html").as_uri() + fragment, wait_until="load")
            page.wait_for_function("document.fonts.status === 'loaded'")
            page.wait_for_timeout(500)
            if fragment == "#save_editor":
                page.locator(".plist .prow:not([data-slot])").first.click()
                page.wait_for_timeout(500)
            cdp = page.context.new_cdp_session(page)
            cdp.send("DOM.enable")
            cdp.send("CSS.enable")
            root = cdp.send("DOM.getDocument", {"depth": -1, "pierce": True})["root"]
            glyphs = {}
            for node, value in text_nodes(root, []):
                try:
                    fonts = cdp.send("CSS.getPlatformFontsForNode", {"nodeId": node})["fonts"]
                except Exception:  # a node the page replaced since the walk
                    continue
                for font in fonts:
                    glyphs[font["familyName"]] = glyphs.get(font["familyName"], 0) + font["glyphCount"]
                    unbundled = {c for c in value if ord(c) not in bundled and not c.isspace()}
                    if not font.get("isCustomFont") and not (unbundled and unbundled <= MEMBER_TEXT):
                        findings.append(f"{fragment or '(shell)'}: {font['familyName']} drew "
                                        f"{font['glyphCount']} glyphs of {value.strip()[:60]!r}")
            # What the page drew, so a run that saw no text cannot pass unnoticed.
            print(f"{fragment or '(shell)'}: " + ", ".join(
                f"{name} {count}" for name, count in sorted(glyphs.items(), key=lambda g: -g[1])))
            cdp.detach()
            page.close()
        browser.close()
    return findings


def main():
    findings = source_findings() + page_findings()
    for finding in findings:
        print(finding)
    print(f"{len(findings)} findings")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
