"""Tests of `perturb_check.py`: `python scripts/review/perturb_check_test.py`."""

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import perturb_check  # noqa: E402

GUARDED = "def value():\n    return 1\n"
TEST = (
    "import unittest\n"
    "import guarded\n"
    "class GuardedTest(unittest.TestCase):\n"
    "    def test_value(self):\n"
    "        self.assertEqual(guarded.value(), 1)\n"
    "unittest.main()\n"
)


class PerturbCheck(unittest.TestCase):
    def check(self, test: str) -> tuple[int, str]:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "guarded.py").write_text(GUARDED, encoding="utf-8")
            (root / "guarded_test.py").write_text(TEST, encoding="utf-8")
            (root / "perturbations.py").write_text(
                f"ROOT = {str(root)!r}\n"
                f"COMMAND = [{sys.executable!r}, 'guarded_test.py']\n"
                f"PERTURBATIONS = [('flip', 'guarded.py', 'return 1', 'return 2', {test!r})]\n",
                encoding="utf-8",
            )
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = perturb_check.main(["perturb_check.py", str(root / "perturbations.py")])
            self.assertEqual((root / "guarded.py").read_text(encoding="utf-8"), GUARDED)
            return code, out.getvalue()

    def test_a_test_that_fails_under_the_perturbation_is_red(self):
        code, out = self.check("GuardedTest.test_value")
        self.assertEqual(code, 0)
        self.assertIn("RED   flip", out)

    def test_a_test_that_does_not_pass_unperturbed_proves_nothing(self):
        # A misspelt or missing test exits nonzero without running: it read as caught.
        with self.assertRaisesRegex(SystemExit, "flip: GuardedTest.test_missing fails"):
            self.check("GuardedTest.test_missing")


if __name__ == "__main__":
    unittest.main()
