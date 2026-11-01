"""`just clef-diff [base]` and `just clef <crate>`: the Clef bug scan.

Asks Cloudflare's Clef decision model (27B, Workers AI `@cf/cloudflare/clef`) one question,
"does this code contain a bug", about each 60-line window of production Rust, and flags a
window at P(true) >= 0.7. A flagged window then gets one request asking, for each of its code
lines, "line N is where the bug is", and the report lists the three likeliest lines.
CONTRIBUTING.md "Clef scan" says where the scan runs and how flags are ruled; the window size,
question and threshold are the ones measured in the 2026-10-06 evaluation (19 of 21 injected
defects caught, 2 of 64 reviewed hunks flagged), so change them only with a new measurement.

- `diff [base]` scans the windows covering the lines changed since `base` (default HEAD) in
  the working tree, untracked files included (`mutants.working_tree()`).
- `crate <name>` (or `all`) tiles the crate's production code: 60-line windows, 20 overlapping.

Production code only: files under a `tests/` folder, `tests.rs` files, modules declared under
`#[cfg(test)]` in a file of their own (`test_support.rs`) and a file's inline test module are
left out (the question was not measured on test code, where the project allows unwraps and
clones).

Rulings: `scripts/review/clef_rulings.md` (tracked) holds the lead's verdict on each flag, keyed by
the flagged file and the five lines around its likeliest line. A flag whose key is ruled is
reported under "Ruled earlier", not as new, so a dismissed flag is not raised again while its
code is unchanged, and an accepted one that is still flagged shows its fix has not landed.

Answers are cached in `clef.out/cache.json` by request content, so a rerun pays only for what
is new. The report is `clef.out/report.md`.

Tokens: `CLOUDFLARE_API_TOKEN`, else the file named by `STUDIO_CLEF_TOKEN_FILE` (read from the
user's registry environment too on Windows, like `STUDIO_MUTANTS_REMOTE`), one API token with
Workers AI permission per line, used in order. Each token's account comes from the token
(`CLOUDFLARE_ACCOUNT_ID` overrides it when there is a single token).

Daily limit: the free tier allows 10,000 neurons a UTC day per account. A token that reaches
it is marked spent for the day in `clef.out/queue.json` (by a hash, not the token) and the next
token takes over; later runs that day skip it without a request. When every token is spent,
the scan is queued in the same file with the code it covers as it is now, and the run exits 0
saying so: work goes on without Clef. Every scan started later that same UTC day is queued
without trying. The first run after the date changes scans the queue first, oldest first, then
its own; its report has one section per scan, each judging the code as it was when queued.
"""

import hashlib
import json
import os
import re
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "mutation"))
import mutants  # noqa: E402

OUT = ROOT / "clef.out"
RULINGS = ROOT / "scripts" / "review" / "clef_rulings.md"
MODEL = "@cf/cloudflare/clef"
WINDOW = 60
STRIDE = 40
THRESHOLD = 0.7
QUESTION = ("Does this Rust code contain a bug: a failure that passes silently, a value that is "
            "truncated or wraps, an out-of-bounds index or slice, a panic on malformed input, or "
            "logic that contradicts its own documentation or names?")
HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")


class ScanError(Exception):
    pass


class QuotaExhausted(ScanError):
    """The free tier's daily neuron allowance is spent until the UTC date changes."""


def env(name: str) -> str | None:
    value = os.environ.get(name)
    if value is None and os.name == "nt":
        import winreg

        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                value = winreg.QueryValueEx(key, name)[0]
        except OSError:
            value = None
    return value or None


def tokens() -> list[str]:
    """`CLOUDFLARE_API_TOKEN`, else the token file's non-empty lines, in order of use."""
    if token := env("CLOUDFLARE_API_TOKEN"):
        return [token]
    if path := env("STUDIO_CLEF_TOKEN_FILE"):
        lines = Path(path).read_text(encoding="utf-8").splitlines()
        found = [line.strip() for line in lines if line.strip() and not line.startswith("#")]
        if found:
            return found
    raise ScanError("no token: set CLOUDFLARE_API_TOKEN or STUDIO_CLEF_TOKEN_FILE")


def token_id(token: str) -> str:
    """A token's name in clef.out/queue.json: a hash, never the token itself."""
    return hashlib.sha256(token.encode("utf-8")).hexdigest()[:12]


class Clef:
    """Workers AI client over one or more tokens (accounts), used in order: a token whose
    account reaches the daily limit is marked spent for the UTC day in `exhausted` (saved with
    the queue, so later runs that day skip it without a request) and the next one takes over.
    QuotaExhausted is raised only when every token is spent. Tokens never reach the output."""

    def __init__(self, exhausted: dict[str, str]) -> None:
        self.slots = [{"label": f"token {n}", "token": t, "account": None}
                      for n, t in enumerate(tokens(), 1)]
        self.exhausted = exhausted
        self.lock = threading.Lock()

    def current(self) -> dict | None:
        today = utc_today()
        return next((s for s in self.slots if self.exhausted.get(token_id(s["token"])) != today),
                    None)

    def request(self, slot: dict, url: str, body: dict | None) -> dict:
        token = slot["token"]
        data = None if body is None else json.dumps(body).encode("utf-8")
        for attempt in range(5):
            req = urllib.request.Request(url, data=data, method="GET" if body is None else "POST",
                                         headers={"Authorization": f"Bearer {token}",
                                                  "Content-Type": "application/json"})
            try:
                with urllib.request.urlopen(req, timeout=120) as r:
                    return json.loads(r.read().decode("utf-8"))
            except urllib.error.HTTPError as e:
                detail = e.read().decode("utf-8", "replace")[:500]
                for each in self.slots:
                    detail = detail.replace(each["token"], "<token>")
                # The free tier's daily neuron allowance (seen as code 4006, "daily free
                # allocation") is a 429 too, but retrying cannot help.
                if "daily free allocation" in detail:
                    raise QuotaExhausted(detail) from None
                if e.code in (429, 500, 502, 503) and attempt < 4:
                    time.sleep(2 * (attempt + 1))
                    continue
                raise ScanError(f"HTTP {e.code}: {detail}") from None
            except urllib.error.URLError as e:
                if attempt < 4:
                    time.sleep(2 * (attempt + 1))
                    continue
                raise ScanError(f"network: {e.reason}") from None
        raise AssertionError("unreachable")

    def lookup_account(self, slot: dict) -> str:
        # `CLOUDFLARE_ACCOUNT_ID` names one account, so it only serves a single token.
        if len(self.slots) == 1 and (account := env("CLOUDFLARE_ACCOUNT_ID")):
            return account
        accounts = self.request(slot, "https://api.cloudflare.com/client/v4/accounts",
                                None)["result"]
        if len(accounts) != 1:
            raise ScanError(f"{slot['label']} sees {len(accounts)} accounts, not one")
        return accounts[0]["id"]

    def ask(self, state: str, questions: dict) -> dict:
        """{question id: P(true)} for noul questions."""
        while True:
            with self.lock:
                slot = self.current()
                if slot is None:
                    raise QuotaExhausted("every token's daily limit is spent")
                if slot["account"] is None:
                    slot["account"] = self.lookup_account(slot)
            url = (f"https://api.cloudflare.com/client/v4/accounts/{slot['account']}"
                   f"/ai/run/{MODEL}")
            try:
                resp = self.request(slot, url, {"state": state, "questions": questions})
            except QuotaExhausted:
                with self.lock:
                    key = token_id(slot["token"])
                    if self.exhausted.get(key) != utc_today():
                        self.exhausted[key] = utc_today()
                        print(f"{slot['label']}: daily neuron limit reached; next token",
                              flush=True)
                continue
            if not resp.get("success", True):
                raise ScanError(json.dumps(resp.get("errors")))
            answers = resp["result"]["answers"]
            return {q: float(a["noul"]) for q, a in answers.items()}


class Cache:
    def __init__(self) -> None:
        path = OUT / "cache.json"
        self.data = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}

    @staticmethod
    def key(state: str, questions: dict) -> str:
        blob = json.dumps([MODEL, state, questions], sort_keys=True).encode("utf-8")
        return hashlib.sha256(blob).hexdigest()

    def save(self) -> None:
        OUT.mkdir(exist_ok=True)
        tmp = OUT / "cache.json.tmp"
        tmp.write_text(json.dumps(self.data), encoding="utf-8")
        os.replace(tmp, OUT / "cache.json")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True,
                          encoding="utf-8", errors="replace").stdout


TEST_MODULE = re.compile(r"\s*(pub(\([^)]*\))?\s+)?mod\s+\w+\s*\{")


def test_only_file(path: str) -> bool:
    """Whether the file is a module its parent declares under `#[cfg(test)]` (`mod name;` on
    the next line), or lies below one: test code with a file of its own (`test_support.rs`,
    a golden test module), which `production_end`'s inline rule cannot see. The parent is
    looked up the two ways Rust allows (`a/mod.rs` or `a.rs`); a file whose parent is not
    found counts as production."""
    p = PurePosixPath(path)
    if p.parent.name == "src" and p.name in ("lib.rs", "main.rs"):
        return False
    name, folder = (p.parent.name, p.parent.parent) if p.name == "mod.rs" else (p.stem, p.parent)
    if folder.name == "src":
        parents = [folder / "lib.rs", folder / "main.rs"]
    else:
        parents = [folder / "mod.rs", folder.with_suffix(".rs")]
    declaration = re.compile(rf"\s*(pub(\([^)]*\))?\s+)?mod\s+{name}\s*;")
    for parent in parents:
        if not (ROOT / parent).is_file():
            continue
        lines = (ROOT / parent).read_text(encoding="utf-8", errors="replace").splitlines()
        for i, line in enumerate(lines):
            if declaration.match(line):
                return (i > 0 and lines[i - 1].strip() == "#[cfg(test)]") or test_only_file(
                    str(parent))
    return False


def production_end(path: str, lines: list[str]) -> int:
    """How many leading lines of the file are production code (0 for a test file): up to the
    inline test module, a `#[cfg(test)]` directly over `mod <name> {`. Not the first
    `#[cfg(test)]`: that also marks out-of-line modules (`mod tests;`) and single items at
    the top or middle of a file (20 files, 5,007 production lines, on 2026-10-06). The
    inline test module is the last item of every file that has one (133 files checked)."""
    if "/tests/" in path or path.endswith("tests.rs") or test_only_file(path):
        return 0
    return next((i for i, s in enumerate(lines[:-1]) if s.strip() == "#[cfg(test)]"
                 and TEST_MODULE.match(lines[i + 1])), len(lines))


def windows_over(start: int, end: int, limit: int) -> list[tuple[int, int]]:
    """1-based inclusive WINDOW-line windows covering lines start..end, within 1..limit."""
    if limit < 1:
        return []
    out = []
    if end - start + 1 <= WINDOW:
        lo = max(1, min((start + end) // 2 - WINDOW // 2, limit - WINDOW + 1))
        out.append((lo, min(limit, lo + WINDOW - 1)))
        return out
    lo = max(1, start - (WINDOW - STRIDE) // 2)
    while True:
        hi = min(limit, lo + WINDOW - 1)
        out.append((lo, hi))
        if hi >= min(limit, end):
            return out
        lo += STRIDE


def job(what: str, windows: list[tuple[str, int, int, list[str]]]) -> dict:
    """A scan as data: what it covers, its windows and the text of their files. Self-contained,
    so a queued scan judges the code as it was when it was queued."""
    files = {path: lines for path, _, _, lines in windows}
    return {"what": what, "queued": None, "files": files,
            "windows": [[path, lo, hi] for path, lo, hi, _ in windows]}


def diff_windows(base: str) -> list[tuple[str, int, int, list[str]]]:
    tree = mutants.working_tree()
    diff = git("diff", "-U0", base, tree, "--", "crates/*.rs")
    changed: dict[str, list[tuple[int, int]]] = {}
    path = None
    for raw in diff.splitlines():
        if raw.startswith("+++ "):
            path = raw[6:] if raw.startswith("+++ b/") else None
        elif (m := HUNK.match(raw)) and path:
            start, count = int(m.group(1)), int(m.group(2) or 1)
            if count:
                changed.setdefault(path, []).append((start, start + count - 1))
    out = []
    for path, ranges in changed.items():
        lines = git("show", f"{tree}:{path}").splitlines()
        limit = production_end(path, lines)
        seen = set()
        for start, end in ranges:
            if start > limit:
                continue
            for lo, hi in windows_over(start, min(end, limit), limit):
                if (lo, hi) not in seen:
                    seen.add((lo, hi))
                    out.append((path, lo, hi, lines))
    return out


def crate_windows(name: str) -> list[tuple[str, int, int, list[str]]]:
    files = git("ls-files", "crates/*.rs").split()
    if name != "all":
        # crates/libs/<name>/..., crates/tools/<name>/..., but crates/studio/... too.
        files = [f for f in files if name in f.split("/")[1:3]]
        if not files:
            raise ScanError(f"no crate named {name} under crates/")
    out = []
    for path in files:
        if "/src/" not in path:
            continue
        lines = (ROOT / path).read_text(encoding="utf-8", errors="replace").splitlines()
        limit = production_end(path, lines)
        out += [(path, lo, hi, lines) for lo, hi in windows_over(1, limit, limit)]
    return out


def state_of(lines: list[str], lo: int, hi: int) -> str:
    return "```rust\n" + "\n".join(lines[lo - 1:hi]) + "\n```"


def locate_request(lines: list[str], lo: int, hi: int) -> tuple[str, dict]:
    """The window with relative line numbers, one question per code line (at most 64, the
    API's limit; a 60-line window always fits). Relative numbers are the measured form."""
    window = lines[lo - 1:hi]
    state = "```rust\n" + "\n".join(f"{n:4}| {t}" for n, t in enumerate(window, 1)) + "\n```"
    questions = {}
    for n, text in enumerate(window, 1):
        t = text.strip()
        if t and not t.startswith("//") and t not in ("{", "}", "(", ")", ";", "};", "});"):
            questions[f"L{n}"] = {"type": "noul", "instructions":
                                  "Decide whether this claim is true: the code has a bug, and "
                                  f"line {n} (`{t[:100]}`) is where it is."}
    assert len(questions) <= 64
    return state, questions


def ruling_key(path: str, lines: list[str], line: int) -> str:
    near = "\n".join(s.strip() for s in lines[max(0, line - 3):line + 2])
    return hashlib.sha1(f"{path}\n{near}".encode("utf-8")).hexdigest()[:16]


def load_rulings() -> dict[str, dict[str, str]]:
    """Key -> {verdict, date, line, p, reason}, from the rulings table's rows."""
    rulings = {}
    for row in RULINGS.read_text(encoding="utf-8").splitlines():
        cells = [c.strip().replace("\\|", "|")
                 for c in re.split(r"(?<!\\)\|", row.strip())[1:-1]]
        if len(cells) == 6 and re.fullmatch(r"[0-9a-f]{16}", cells[0]):
            rulings[cells[0]] = dict(zip(("verdict", "date", "line", "p", "reason"), cells[1:]))
    return rulings


def utc_today() -> str:
    return time.strftime("%Y-%m-%d", time.gmtime())


def write_atomic(path: Path, text: str) -> None:
    OUT.mkdir(exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    os.replace(tmp, path)


def load_queue() -> dict:
    """{quota_date: the UTC day every token was spent, or None; jobs: queued scans;
    exhausted: token id -> the UTC day it was spent, today's entries only}."""
    path = OUT / "queue.json"
    queue = {"quota_date": None, "jobs": [], "exhausted": {}}
    if path.exists():
        queue.update(json.loads(path.read_text(encoding="utf-8")))
    today = utc_today()
    queue["exhausted"] = {k: d for k, d in queue["exhausted"].items() if d == today}
    return queue


def save_queue(queue: dict) -> None:
    write_atomic(OUT / "queue.json", json.dumps(queue))


def scan(job: dict, ask) -> list[tuple]:
    """The job's flags: (path, lo, hi, P, [(line, P)] likeliest lines first)."""
    windows = [(path, lo, hi, job["files"][path]) for path, lo, hi in job["windows"]]
    question = {"bug": {"type": "noul", "instructions": QUESTION}}
    with ThreadPoolExecutor(max_workers=4) as pool:
        scores = list(pool.map(lambda w: ask(state_of(w[3], w[1], w[2]), question)["bug"],
                               windows))
        flagged = [(w, p) for w, p in zip(windows, scores) if p >= THRESHOLD]
        requests = [locate_request(w[3], w[1], w[2]) for w, _ in flagged]
        # A window with no code line (all comments) has nothing to point at.
        located = list(pool.map(lambda r: ask(*r) if r[1] else {}, requests))
    flags = []
    for ((path, lo, hi, _), p), where in zip(flagged, located):
        top = sorted(where, key=lambda q: -where[q])[:3]
        flags.append((path, lo, hi, p, [(lo + int(q[1:]) - 1, where[q]) for q in top]))
    return flags


def merge_overlapping(flags: list[tuple]) -> list[tuple]:
    """One flag per run of overlapping flagged windows in a file: overlapping windows usually
    flag one issue (the 2026-10-06 measurement: 10 flagged windows, three issues), and a flag
    each would mean ruling it several times. The merged flag keeps the highest P and the three
    likeliest distinct lines across its windows."""
    merged = []
    for path, lo, hi, p, rows in sorted(flags, key=lambda f: (f[0], f[1])):
        if merged and merged[-1][0] == path and lo <= merged[-1][2]:
            last = merged[-1]
            lines = dict(last[4])
            for line, q in rows:
                lines[line] = max(q, lines.get(line, 0.0))
            top = sorted(lines.items(), key=lambda lq: -lq[1])[:3]
            merged[-1] = (path, last[1], max(hi, last[2]), max(p, last[3]), top)
        else:
            merged.append((path, lo, hi, p, rows))
    return merged


def report_section(job: dict, flags: list[tuple], rulings: dict) -> tuple[list[str], int]:
    """The job's report lines and its count of new flags."""
    when = f", queued on {job['queued']} (the code as it was then)" if job["queued"] else ""
    flagged_windows = len(flags)
    flags = merge_overlapping(flags)
    new, ruled = [], []
    for path, lo, hi, p, rows in flags:
        lines = job["files"][path]
        key = ruling_key(path, lines, rows[0][0]) if rows else ""
        (ruled if key in rulings else new).append((path, lo, hi, p, rows, key, lines))
    out = [f"## {job['what']}{when}", "",
           f"{len(job['windows'])} windows of production code, {flagged_windows} flagged at "
           f"P >= {THRESHOLD}, {len(flags)} flags once overlaps are merged: {len(new)} new, "
           f"{len(ruled)} ruled earlier.", ""]
    for title, entries in (("New flags", new), ("Ruled earlier", ruled)):
        out += [f"### {title}", ""]
        for path, lo, hi, p, rows, key, lines in sorted(entries, key=lambda e: -e[3]):
            out.append(f"- **{path}:{lo}-{hi}**, P {p:.2f}")
            for line, q in rows:
                out.append(f"  - {q:.2f} line {line}: `{lines[line - 1].strip()[:110]}`")
            if key in rulings:
                r = rulings[key]
                out.append(f"  - ruled {r['verdict']} on {r['date']}: {r['reason']}")
            elif key:
                out.append(f"  - row: `| {key} | accepted/rejected | YYYY-MM-DD | "
                           f"{path}:{rows[0][0]} | {p:.2f} | reason |`")
        if not entries:
            out.append("None.")
        out.append("")
    return out, len(new)


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[1] not in ("diff", "crate") or (argv[1] == "crate" and len(argv) < 3):
        print("usage: clef_scan.py diff [base] | crate <name|all>")
        return 2
    try:
        if argv[1] == "diff":
            base = argv[2] if len(argv) > 2 else "HEAD"
            current = job(f"diff from {base} at {git('rev-parse', '--short', 'HEAD').strip()}",
                          diff_windows(base))
        else:
            current = job(f"crate {argv[2]}", crate_windows(argv[2]))
    except ScanError as e:
        print(f"clef scan failed: {e}")
        return 1
    print(f"clef scan, {current['what']}: {len(current['windows'])} windows", flush=True)

    queue, today = load_queue(), utc_today()
    if queue["quota_date"] == today:
        if current["windows"]:
            current["queued"] = today
            queue["jobs"].append(current)
            save_queue(queue)
        write_atomic(OUT / "report.md", f"# Clef scan\n\nQUEUED: the daily neuron limit was "
                     f"reached today (UTC). {len(queue['jobs'])} scans wait for the next run "
                     "after the date changes.\n")
        print(f"QUEUED: the daily neuron limit was reached today (UTC); {len(queue['jobs'])} "
              "scans wait for the next run after the date changes. Work continues without Clef.")
        return 0

    cache, rulings, clef, made = Cache(), load_rulings(), None, threading.Lock()

    def ask(state: str, questions: dict) -> dict:
        nonlocal clef
        key = Cache.key(state, questions)
        if key not in cache.data:
            with made:
                # Made on the first uncached question only: a fully cached run needs no token.
                clef = clef or Clef(queue["exhausted"])
            cache.data[key] = clef.ask(state, questions)
        return cache.data[key]

    jobs = queue["jobs"] + [current]
    sections, new_total, done, failure = [], 0, 0, None
    try:
        for done, each in enumerate(jobs):
            lines, new = report_section(each, scan(each, ask), rulings)
            sections += lines
            new_total += new
        done = len(jobs)
        queue["quota_date"], queue["jobs"] = None, []
    except QuotaExhausted:
        for each in jobs[done:]:
            each["queued"] = each["queued"] or today
        queue["quota_date"], queue["jobs"] = today, jobs[done:]
    except ScanError as e:
        # Not the quota: the queued scans stay queued, the current one is rerun by hand.
        queue["jobs"] = jobs[done:-1]
        failure = e
    finally:
        cache.save()
        save_queue(queue)

    header = ["# Clef scan", "",
              "Each flag is ruled like a reviewer concern (CONTRIBUTING.md \"Clef scan\"); its "
              "row goes into `scripts/review/clef_rulings.md`.", ""]
    waiting = len(jobs) - done
    if waiting and failure is None:
        header += [f"QUEUED: the daily neuron limit was reached; {waiting} scans (this run's "
                   "included) wait for the next run after the UTC date changes.", ""]
    write_atomic(OUT / "report.md", "\n".join(header + sections))
    if failure is not None:
        print(f"clef scan failed: {failure}")
        print("answers received so far are cached; a rerun pays only for the rest")
        return 1
    print(f"{done} scans done, {new_total} new flags; report in clef.out/report.md")
    if waiting:
        print(f"QUEUED: the daily neuron limit was reached; {waiting} scans wait for the next "
              "run after the UTC date changes. Work continues without Clef.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
