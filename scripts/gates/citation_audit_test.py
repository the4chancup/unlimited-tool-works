"""Tests of scripts/gates/citation_audit.py's offline parts (no request is sent); `just acceptance`
runs them."""

import io
import json
import unittest
import urllib.error
from unittest import mock

from acceptance import Scan, scan_plan, scan_source
from citation_audit import (RULING_ROW, THRESHOLD, AuditError, Keys, Pair, ask, cited_test, pairs, report,
                            ruled_keys, scenario_text, state)

PLAN = """# Tool

## Acceptance

```
TC-KIT-01  GIVEN a kit
           WHEN it is compiled
           THEN it is written
TC-KIT-02  GIVEN two kits
           WHEN they are compiled
           THEN both are written
TC-KIT-03  withdrawn: merged into TC-KIT-02
```
"""

SOURCE = """use super::*;

fn compile_one(name: &str) -> Run {
    run(&["compile", name])
}

// TC-KIT-01
#[test]
fn a_kit_is_written() {
    let run = compile_one("p1");
    assert!(run.wrote("p1"));
}

// TC-KIT-01
// TC-KIT-02
#[test]
fn two_kits_are_written() {
    assert!(compile_one("p1").wrote("p1"));
    if true {
        assert!(compile_one("p2").wrote("p2"));
    }
}
"""

PATH = "crates/tools/t/tests/cli/kits.rs"


def scan_and_files():
    scan = Scan()
    scan_plan(scan, "docs/spec/t.md", PLAN)
    scan_source(scan, PATH, SOURCE)
    files = {"docs/spec/t.md": PLAN.splitlines(), PATH: SOURCE.splitlines()}
    return scan, files.__getitem__


class ExtractionTests(unittest.TestCase):
    def test_a_scenario_runs_to_the_next_id_line(self):
        lines = PLAN.splitlines()
        start = lines.index(next(line for line in lines if line.startswith("TC-KIT-01")))
        self.assertEqual(scenario_text(lines, start).splitlines()[-1].strip(), "THEN it is written")

    def test_a_cited_test_runs_from_its_first_citation_through_its_closing_brace(self):
        lines = SOURCE.splitlines()
        citation = lines.index("// TC-KIT-02")
        name, source = cited_test(lines, citation)
        self.assertEqual(name, "two_kits_are_written")
        self.assertTrue(source.startswith("// TC-KIT-01\n// TC-KIT-02\n#[test]"))
        self.assertTrue(source.endswith("    }\n}"), source)

    def test_braces_in_strings_chars_and_comments_do_not_end_a_test(self):
        lines = [
            "// TC-KIT-01",
            "#[test]",
            "fn a_rules_file_is_written() {",
            '    let text = "fn f() {};";',
            '    let raw = r#"}"# ;',
            "    let open = '{';",
            "    // a } in a comment",
            '    let escaped = "\\"}";',
            "    let multi = \"}",
            '    }";',
            "    assert!(written(text));",
            "}",
            "fn after() {}",
        ]
        name, source = cited_test(lines, 0)
        self.assertEqual(name, "a_rules_file_is_written")
        self.assertTrue(source.endswith("    assert!(written(text));\n}"), source)

    def test_a_citation_not_above_a_test_gives_no_test(self):
        self.assertIsNone(cited_test(["// TC-KIT-01", "fn helper() {}"], 0))


class PairTests(unittest.TestCase):
    def test_each_citation_is_one_pair_with_the_test_s_own_helpers(self):
        found = pairs(*scan_and_files())
        self.assertEqual(
            sorted((p.scenario, p.test.split("::")[1]) for p in found),
            [("TC-KIT-01", "a_kit_is_written"), ("TC-KIT-01", "two_kits_are_written"),
             ("TC-KIT-02", "two_kits_are_written")],
        )
        state = next(p for p in found if p.test.endswith("a_kit_is_written")).state
        self.assertIn("THEN it is written", state)
        self.assertIn("fn compile_one(name: &str) -> Run {", state, "the helper it calls")
        self.assertNotIn("THEN both are written", state)

    def test_a_pair_s_key_changes_with_its_state(self):
        self.assertNotEqual(Pair("A", "t", "x").key, Pair("A", "t", "y").key)
        self.assertEqual(Pair("A", "t", "x").key, Pair("B", "u", "x").key)


class ReportTests(unittest.TestCase):
    def test_only_unruled_pairs_under_the_threshold_are_listed(self):
        def pair(scenario, test):
            return Pair(scenario, test, state(f"{scenario} GIVEN x", "p.rs", f"fn {test}() {{}}", []))

        low, ruled, high = pair("TC-KIT-01", "a"), pair("TC-KIT-01", "b"), pair("TC-KIT-02", "c")
        text = report([(low, THRESHOLD - 0.1), (ruled, 0.01), (high, THRESHOLD)], {ruled.key})
        self.assertIn("3 citations asked, 2 under P 0.3: 1 new, 1 already ruled", text)
        self.assertIn("## TC-KIT-01 ← a (P 0.200)", text)
        self.assertNotIn("← b", text)
        self.assertNotIn("← c", text)

    def test_a_ruling_row_is_read_by_its_key(self):
        rows = "| Key | Verdict |\n|---|---|\n| 0123456789abcdef | rejected | 2026-10-17 | TC-KIT-01 | a | 0.10 | why |\n"
        self.assertEqual(ruled_keys(rows), {"0123456789abcdef"})
        self.assertIsNone(RULING_ROW.match("| 0123 | rejected |"))


class KeyTests(unittest.TestCase):
    """`ask` against a stand-in service (`urlopen` replaced): no request leaves the machine."""

    def serve(self, refused: dict[str, int]):
        sent = []

        def urlopen(request, timeout):
            key = request.get_header("Authorization").removeprefix("Bearer ")
            sent.append(key)
            if key in refused:
                raise urllib.error.HTTPError(request.full_url, refused[key], "refused", {}, io.BytesIO(b"{}"))
            return io.BytesIO(json.dumps({"answers": {"proves": {"noul": 0.75}}}).encode("utf-8"))

        patcher = mock.patch("urllib.request.urlopen", urlopen)
        patcher.start()
        self.addCleanup(patcher.stop)
        return sent

    def test_a_refused_key_is_left_for_the_next_for_the_rest_of_the_run(self):
        sent = self.serve({"first": 402})
        keys = Keys(["first", "second"])
        self.assertEqual(ask(keys, "state"), 0.75)
        self.assertEqual(ask(keys, "state"), 0.75)
        self.assertEqual(sent, ["first", "second", "second"])

    def test_the_last_keys_refusal_is_the_runs_error(self):
        sent = self.serve({"first": 403, "second": 401})
        with self.assertRaises(AuditError) as raised:
            ask(Keys(["first", "second"]), "state")
        self.assertEqual(sent, ["first", "second"])
        self.assertIn("HTTP 401", str(raised.exception))
        self.assertNotIn("second", str(raised.exception), "the key is never printed")

    def test_a_request_error_is_not_a_spent_key(self):
        sent = self.serve({"first": 400})
        with self.assertRaises(AuditError):
            ask(Keys(["first", "second"]), "state")
        self.assertEqual(sent, ["first"])


if __name__ == "__main__":
    unittest.main()
