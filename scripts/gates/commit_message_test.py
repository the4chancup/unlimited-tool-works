"""Tests of scripts/gates/commit_message.py; `just test` runs them."""

import unittest

from commit_message import MAX_SUBJECT, problems

GOOD = "fix(vtree): keep a scope's children in insertion order\n\nA sort by name moved the kits.\n"


class Problems(unittest.TestCase):
    def test_a_message_of_the_projects_shape_passes(self) -> None:
        self.assertEqual(problems(GOOD), [])
        self.assertEqual(problems("merge(docs): the readability pass over the core plans"), [])
        self.assertEqual(problems("docs(plans, changelog): name the launcher's tally\n"), [])
        # The blank lines around a message read from a file are git's to drop.
        self.assertEqual(problems("\n\n" + GOOD + "\n\n"), [])

    def test_a_first_line_without_a_scope_is_refused(self) -> None:
        found = problems("docs: b07 pass 2 of the readability pass")
        self.assertEqual(found, ["the first line is not `type(scope): summary`"])

    def test_a_type_outside_the_list_is_named(self) -> None:
        found = problems("build(gates): give the tests a linked temp folder")
        self.assertEqual(len(found), 1)
        self.assertIn("the type `build` is not one of feat, fix", found[0])

    def test_the_first_line_stops_at_120_characters(self) -> None:
        head = "docs(plans): "
        at_the_limit = head + "x" * (MAX_SUBJECT - len(head))
        self.assertEqual(problems(at_the_limit), [])
        found = problems(at_the_limit + "x")
        self.assertEqual(len(found), 1)
        self.assertIn("121 characters, over 120", found[0])

    def test_a_full_stop_or_a_space_at_the_end_of_the_first_line_is_refused(self) -> None:
        for subject in ("fix(vtree): keep the order.", "fix(vtree): keep the order "):
            self.assertEqual(problems(subject), ["the first line ends with a full stop or a space"])

    def test_the_body_starts_after_one_blank_line(self) -> None:
        found = problems("fix(vtree): keep the order\nA sort by name moved the kits.")
        self.assertEqual(found, ["the second line is not blank: the body starts after one empty line"])

    def test_a_line_crediting_an_agent_is_refused_wherever_it_stands(self) -> None:
        for credit in ("Co-Authored-By: Claude <noreply@anthropic.com>",
                       "co-authored-by: someone", "🤖 Generated with [Claude Code](x)"):
            found = problems(GOOD + "\n" + credit + "\n")
            self.assertEqual(found, [f"the line `{credit}` credits an agent"], credit)

    def test_an_empty_message_is_one_problem(self) -> None:
        self.assertEqual(problems(""), ["the message is empty"])
        self.assertEqual(problems("\n  \n"), ["the message is empty"])

    def test_every_problem_of_a_message_is_listed(self) -> None:
        found = problems("Update stuff." + "\nmore\n\nCo-Authored-By: x")
        self.assertEqual(len(found), 4, found)


if __name__ == "__main__":
    unittest.main()
