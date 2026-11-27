"""Tests of scripts/pages/inbox.py; `just test` runs them."""

import re
import tempfile
import unittest
import unittest.mock
from pathlib import Path

import inbox

TODAY = "2026-11-27"
NEXT = "2026-11-28"


def page(pairs: list[tuple[str, str]], sample: tuple[str, str] | None = None) -> str:
    """A minimal maintainer page holding `pairs` of (id, h3 text) entries, `sample` an
    entry inside the page's top comment, which is not one."""
    parts = ["<!doctype html>"]
    if sample:
        parts.append(f'<!--\nAn entry: <section class="entry" id="{sample[0]}"> '
                     f'with an <h3>{sample[1]}</h3>\n-->')
    parts += [f'<section class="entry" id="{entry_id}">\n  <h3>{title}</h3>\n</section>'
              for entry_id, title in pairs]
    return "\n".join(parts) + "\n"


class Main(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.pages = Path(self.dir.name)
        self.write("questions.html", page([("alpha", "The first question"),
                                           ("beta", "The second question")],
                                          sample=("sample-q", "Not an entry")))
        self.write("proposals.html", page([("p1", "The first proposal")]))
        self.write("tasks.html", page([]))
        self.write("ui_decisions.html", page([]))

    def write(self, name: str, text: str) -> None:
        (self.pages / name).write_text(text, encoding="utf-8", newline="\n")

    def run_main(self, day: str = TODAY) -> str:
        """`inbox.main()` over `self.pages`, `day` the date new pointers land; the page
        it writes."""
        keep = inbox.DIR
        inbox.DIR = str(self.pages)
        try:
            with unittest.mock.patch.object(inbox, "today", return_value=day):
                inbox.main()
            return (self.pages / "inbox.html").read_text(encoding="utf-8")
        finally:
            inbox.DIR = keep

    def keys(self, out: str) -> list[tuple[str, str]]:
        """(key, landed) per pointer `li` on the page, in the page's order."""
        return re.findall(r'<li data-key="([^"]+)" data-landed="([^"]+)"', out)

    def test_a_first_run_points_to_every_entry_landed_today(self) -> None:
        out = self.run_main()
        self.assertEqual(self.keys(out), [("questions#alpha", TODAY),
                                          ("questions#beta", TODAY),
                                          ("proposals#p1", TODAY)])
        self.assertIn('<a href="questions.html#alpha">The first question</a>', out)
        self.assertIn('<a href="proposals.html#p1">The first proposal</a>', out)
        self.assertEqual(out.count('class="noticed"'), 3)
        self.assertNotIn("sample-q", out)

    def test_a_second_run_keeps_the_dates_and_lands_a_new_entry_today(self) -> None:
        self.run_main()
        self.write("questions.html", page([("alpha", "The first question"),
                                           ("beta", "The second question"),
                                           ("gamma", "A new question")]))
        out = self.run_main(NEXT)
        self.assertEqual(self.keys(out), [("questions#gamma", NEXT),
                                          ("questions#alpha", TODAY),
                                          ("questions#beta", TODAY),
                                          ("proposals#p1", TODAY)])

    def test_an_entry_off_its_page_loses_its_pointer(self) -> None:
        self.run_main()
        self.write("questions.html", page([("beta", "The second question")]))
        out = self.run_main(NEXT)
        self.assertEqual(self.keys(out), [("questions#beta", TODAY),
                                          ("proposals#p1", TODAY)])

    def test_the_same_id_on_both_pages_gets_two_pointers(self) -> None:
        self.write("questions.html", page([("dup", "A question")]))
        self.write("proposals.html", page([("dup", "A proposal")]))
        out = self.run_main()
        self.assertEqual(self.keys(out), [("questions#dup", TODAY),
                                          ("proposals#dup", TODAY)])
        self.assertIn('<a href="questions.html#dup">A question</a>', out)
        self.assertIn('<a href="proposals.html#dup">A proposal</a>', out)

    def test_entries_on_tasks_and_ui_decisions_get_their_pages_pointer(self) -> None:
        self.write("tasks.html", page([("t1", "A task")]))
        self.write("ui_decisions.html", page([("u1", "A UI decision")]))
        out = self.run_main()
        self.assertEqual(self.keys(out), [("questions#alpha", TODAY),
                                          ("questions#beta", TODAY),
                                          ("proposals#p1", TODAY),
                                          ("tasks#t1", TODAY),
                                          ("ui_decisions#u1", TODAY)])
        self.assertIn('<a href="tasks.html#t1">A task</a> '
                      '<span class="from">Task, landed', out)
        self.assertIn('<a href="ui_decisions.html#u1">A UI decision</a> '
                      '<span class="from">UI decision, landed', out)

    def test_a_title_is_escaped_and_no_entry_says_nothing_new(self) -> None:
        self.write("questions.html", page([("weird", "a < b & c")]))
        self.write("proposals.html", page([]))
        out = self.run_main()
        self.assertIn("a &lt; b &amp; c", out)
        self.write("questions.html", page([]))
        out = self.run_main(NEXT)
        self.assertIn("<p>Nothing new.</p>", out)
        self.assertNotIn("<li ", out)


if __name__ == "__main__":
    unittest.main()
