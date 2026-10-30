The Ruleset editor's window builds an AATF ruleset through forms and saves it as a
`.4ccrules` rules file the Save editor and the Team creator can check against, like the
official one.

**Start page.** With no ruleset open the window offers to start one: *New from
template* (the official rules and the VGL, VTL and DBC cup rulesets, then *Blank*,
one tier, no brackets, the universal rules on), *Open file* to pick an existing
rules file, and the recently used files below, newest first. A file that does not
open lists what is wrong with it under the buttons.

**Editing.** The sections sit in a rail on the left; Custom checks joins only when
the file carries hand-written checks, and a "⚠" marks a section holding a problem
(the bar along the bottom names it). *General* holds the ruleset's name, its event's
page and logo, the squad size and the stat checklists: the rating stats pick the
player's tier, the "may sit below the target" stats are the only ones allowed under
theirs. The logo shows at the Save editor's top right while the ruleset is active, a
link to the page. The other
sections' forms arrive in a later build; the Summary already spells the ruleset
out as readable rules text, with **Copy** putting it on the clipboard to post.
*Show preview* at the top right opens the rules file's data block, as the editor
writes it, with the lines the last edit changed highlighted.

The File menu's *New from template…* brings the start page back over the open
ruleset. Undo and redo (Edit menu, Ctrl+Z and Ctrl+Y or Ctrl+Shift+Z) step through
the edits. A file whose checking code is not this build's, or is an older official
version, shows a banner under the toolbar: the first kind is kept as it is when
saving, the second offers *Update logic*.

**Saving.** *Save* writes the rules file (Save as lets you choose where), but
never an invalid one: it lists what blocks the write instead. Closing the window,
opening another file or picking a template with unsaved changes asks whether to
save them first.
