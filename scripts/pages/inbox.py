"""Writes `docs/maintainer/inbox.html`: one pointer per card on the maintainer's card
pages (`questions.html`, `proposals.html`, `tasks.html`, `ui_decisions.html`), newest
landed first, so a card that lands while its page goes unread is still seen. Each
pointer links to its card and carries a Noticed button that hides it in that browser
(`maintainer.js` keeps the noticed keys in localStorage, the way the pages keep their
picks). A pointer keeps the date it first landed and leaves the file once its card has
left its page. `scripts/commit.py` runs the build for every commit that changes one of
the pages; it also runs by hand as `python scripts/pages/inbox.py` from the
repository's root. The page is never edited by hand."""
import datetime
import html
import os
from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path

DIR = "docs/maintainer"
# The pages whose cards the inbox points at, in the order tied pointers take.
PAGES = {"questions": "Question", "proposals": "Proposal",
         "tasks": "Task", "ui_decisions": "UI decision"}


@dataclass
class Card:
    """One pointer on the page: `key` (`<page>#<id>`, what the browser remembers as
    noticed), `landed` (the `YYYY-MM-DD` the pointer first appeared), `page`, `id` and
    `title` (the card's heading)."""
    key: str
    landed: str
    page: str
    id: str
    title: str


class Entries(HTMLParser):
    """Each `section.entry`'s `(id, h3 text)` of one page, in the page's order. The
    sample entry in a page's top comment is never seen: the parser reports comments
    through `handle_comment`, which is not overridden."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.entries: list[tuple[str, str]] = []
        self._depth = 0
        self._id: str | None = None
        self._h3 = False
        self._text: list[str] = []

    def handle_starttag(self, tag: str, attrs: list) -> None:
        attrs = dict(attrs)
        if tag == "section":
            self._depth += 1
            if self._id is None and "entry" in attrs.get("class", "").split():
                self._id = attrs.get("id")
        elif tag == "h3" and self._id is not None:
            self._h3 = True

    def handle_data(self, data: str) -> None:
        if self._h3:
            self._text.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag == "h3":
            self._h3 = False
        elif tag == "section" and self._depth:
            self._depth -= 1
            if not self._depth and self._id is not None:
                title = " ".join("".join(self._text).split())
                self.entries.append((self._id, title))
                self._id, self._text = None, []


class Pointers(HTMLParser):
    """Each pointer `li`'s key and landed date, from the page being replaced: the
    build's memory of when each card's pointer first appeared."""

    def __init__(self) -> None:
        super().__init__()
        self.landed: dict[str, str] = {}

    def handle_starttag(self, tag: str, attrs: list) -> None:
        if tag == "li":
            attrs = dict(attrs)
            if "data-key" in attrs and "data-landed" in attrs:
                self.landed[attrs["data-key"]] = attrs["data-landed"]


def today() -> str:
    """Today's date (local), `YYYY-MM-DD`: when a new pointer lands."""
    return datetime.date.today().isoformat()


def cards(day: str, landed: dict[str, str]) -> list[Card]:
    """A `Card` per entry on the two pages, newest landed first; ties keep the pages'
    order. An entry the file being replaced never pointed at lands `day`."""
    found = []
    for page in PAGES:
        parser = Entries()
        parser.feed((Path(DIR) / f"{page}.html").read_text(encoding="utf-8"))
        for entry_id, title in parser.entries:
            key = f"{page}#{entry_id}"
            found.append(Card(key, landed.get(key, day), page, entry_id, title))
    found.sort(key=lambda card: card.landed, reverse=True)
    return found


NAV = """\
<nav class="pages">
  <a class="here" href="inbox.html">Inbox</a>
  <a href="roadmap.html">Roadmap</a>
  <a href="questions.html">Questions</a>
  <a href="proposals.html">Proposals</a>
  <a href="tasks.html">Tasks</a>
  <a href="ui_decisions.html">UI decisions</a>
</nav>"""


def render(cards: list[Card]) -> str:
    """The page: one `li` per card, each its link, its page's name, its landed date and
    a `button.noticed`; a "Nothing new." line when no card waits."""
    if cards:
        items = "\n".join(
            f'<li data-key="{html.escape(card.key, quote=True)}" '
            f'data-landed="{html.escape(card.landed, quote=True)}">'
            f'<a href="{card.page}.html#{html.escape(card.id, quote=True)}">'
            f"{html.escape(card.title, quote=False)}</a> "
            f'<span class="from">{PAGES[card.page]}, landed '
            f'{html.escape(card.landed, quote=False)}</span> '
            '<button class="noticed" type="button">Noticed</button></li>'
            for card in cards)
        listing = f"<ul>\n{items}\n</ul>"
    else:
        listing = "<p>Nothing new.</p>"
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Unlimited Tool Works: inbox</title>
<link rel="stylesheet" href="maintainer.css">
<!--
For the lead: one pointer per card on the maintainer's card pages (questions.html,
proposals.html, tasks.html, ui_decisions.html), newest first, so a card that lands
while its page goes unread is still seen. scripts/pages/inbox.py writes the page, which
is never edited by hand: scripts/commit.py runs it on every commit that changes one of
the pages (a merge rebuilds nothing), and it runs by hand as `python
scripts/pages/inbox.py` from the repository's root. A pointer keeps the date it
landed; it leaves the file once its card has left its page.
-->
</head>
<body data-page="inbox">
{NAV}

<h1>Inbox</h1>
<p class="intro">New cards on the Questions, Proposals, Tasks and UI decisions pages,
newest first. Press <b>Noticed</b> once you have seen a card, and its pointer leaves this
list; the card itself stays on its page until you deal with it.</p>

{listing}
<script src="maintainer.js"></script>
</body>
</html>
"""


def main() -> None:
    out = Path(DIR) / "inbox.html"
    landed = {}
    if out.is_file():
        pointers = Pointers()
        pointers.feed(out.read_text(encoding="utf-8"))
        landed = pointers.landed
    found = cards(today(), landed)
    temp = Path(str(out) + ".new")
    temp.write_text(render(found), encoding="utf-8", newline="\n")
    os.replace(temp, out)
    print(f"{len(found)} pointers")


if __name__ == "__main__":
    main()
