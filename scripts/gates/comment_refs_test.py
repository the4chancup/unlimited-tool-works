"""Tests for comment_refs.py: what each pattern lists and what it lets through."""

import unittest

from comment_refs import hits_in


class CommentRefsTest(unittest.TestCase):
    def kinds(self, path: str, text: str) -> list[str]:
        return [kind for _, kind, _ in hits_in(path, text)]

    def test_each_kind_of_pointer_is_listed(self):
        text = "\n".join(
            [
                '// (core plan, "Phase 1"; see the window paragraph)',
                "/// written as the plan's table says",
                "//! `plans/save_editor.md` \"Tactics editor\"",
                "// DECISIONS 2026-10-27 \"No narrow-pane fallback\"",
                "// the worklog's open issue",
                "// 2.17f's conversion rules",
                "// the 0.0.1m2 audit",
                "// Round S4's S4a-2, the row's band",
                "// the sidekick's contradiction",
                '//! Stages 3 and 4 (`team_compiler/pipeline.md` "3. Per-model-folder")',
                "// as `gui.md` says",
                "// measured at 3.w",
                "// 3.7's diff runs",
                "// moved here at step 4.18",
                "// the hand split of 4.14e3",
                "// excluded since mutants-diff 95f20eac",
                "// the comparison the brief specifies",
                "// (not the brief's row byte 10, asserted)",
            ]
        )
        self.assertEqual(
            self.kinds("crates/x/src/lib.rs", text),
            [
                "plan", "plan", "plan", "decisions", "worklog", "step", "step", "review", "review",
                "document", "document", "step", "step", "step", "step", "history", "review",
                "review",
            ],
        )

    def test_who_decided_and_when_is_listed(self):
        text = "\n".join(
            [
                "/// — no headings, so every panel lines up (the",
                "/// maintainer, 2026-10-26). Each row a drag handle",
                "// the r1 shape the",
                "// maintainer rejected), and wrapped across the list",
                "# PES 19's slots (the maintainer's, 2026-10-10) are the last entries",
            ]
        )
        self.assertEqual(
            self.kinds("crates/x/src/lib.rs", text), ["attribution", "attribution"]
        )
        self.assertEqual(self.kinds("scripts/x.py", text.splitlines()[4]), ["attribution"])

    def test_the_maintainers_files_and_machine_pass(self):
        text = "\n".join(
            [
                "// starter's name in the maintainer's PES 2021 save, which ran 50 points",
                "/// The maintainer's desk: an FHD primary at the origin at 100%",
                "// the maintainer's value, written out so a slipped shift fails",
            ]
        )
        self.assertEqual(self.kinds("crates/x/src/lib.rs", text), [])

    def test_a_toml_or_python_comment_is_read_after_its_hash(self):
        self.assertEqual(self.kinds("Cargo.toml", '# (core plan, "Phase 1")'), ["plan"])
        self.assertEqual(self.kinds("scripts/x.py", "    # 5.6e's rule"), ["step"])

    def test_what_the_rule_allows_passes(self):
        text = "\n".join(
            [
                '// spec/core/gui.md "Scaling with the width" says why',
                "// the draft's `.lrow` spans the list",
                "// ST-WIN-02",
                "// egui 0.37.0 applies the zoom at the next pass",
                "// a planned tool, an airplane, the plane's normal",
                "// 1.5 points wider",
                "// within 1.3e-5 of the body's",
                "// 2.5 times the width; at 1.5 points",
                "// egui 0.37's roles, egui 0.36.1's widgets",
                "// `pre-alpha` for 0.0.x, `alpha` for 0.y.z",
                "// BAG VGL18.5's `player_XXX08.jpg` migrated",
                '// docs/spec/core/gui.md "Shell layout"',
                '# cargo-mutants configuration (CONTRIBUTING.md "Mutation runs")',
                "// the provenance in tests/fixtures/README.md",
                'let s = "plans/x.md"; // not a whole-line comment',
            ]
        )
        self.assertEqual(self.kinds("crates/x/src/lib.rs", text), [])

    def test_code_is_not_read_as_a_comment(self):
        self.assertEqual(self.kinds("crates/x/src/lib.rs", 'let path = "plans/x.md";'), [])
        self.assertEqual(self.kinds("Cargo.toml", 'note = "the core plan"'), [])

    def test_a_comment_matching_two_kinds_is_listed_once(self):
        hits = hits_in("crates/x/src/lib.rs", "// plans/x.md and DECISIONS")
        self.assertEqual([(line, kind) for line, kind, _ in hits], [(1, "plan")])


if __name__ == "__main__":
    unittest.main()
