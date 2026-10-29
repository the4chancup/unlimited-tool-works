The Ruleset editor's commands run from a terminal opened in the folder that holds
`UTW.exe`. A rules file (a `.4ccrules` file) is what a cup's teams are checked
against: tiers, stat targets, heights, cards and special rules, plus the code that runs
the check.

```text
UTW ruleset-editor validate <file>
UTW ruleset-editor update-logic <file>
UTW ruleset-editor summary <file> [-o <out>]
```

`validate` checks that a rules file loads: the text parses, the rules hold together (no
empty names or doubled ids, every id named exists, the tier counts fit the squad, the
height brackets ascend), and the checking code runs on a sample team. A file that passes
prints nothing and exits 0. A file that fails prints one `rules_finding` line per
problem: where it failed when the text is broken (a line and column), or which entry
broke a rule (a path like `tiers[2].free.tricks`); the exit code is 1. A file that
cannot be read at all is refused with exit code 2. Nothing is written either way.

`update-logic` refreshes a file's checking code when it is an older official version:
the file keeps working as it is until you run this, which writes this build's code in
and first copies the file it replaces to `<file>.bak`. A file already carrying this
build's code is left alone (`logic_already_current`, exit 0). A file whose checking code
was edited by hand is refused: replacing it would discard someone's code. A file that
does not load is refused as well; run `validate` on it to see why.

`summary` prints the ruleset as readable rules text, the tiers, heights, card groups and
special rules spelled out, ready to post as an invitational's rules. With `-o` the text is
written to a file instead of printed, and the file it replaces is kept as `<out>.bak`;
run it on two rules files and diff the outputs to compare them. A file that does not load
is refused with exit code 2 before anything is written.

Each command ends with an exit code scripts can test: 0 when it did its work, 1 when it
did its work but reported an error, 2 when it was refused before writing anything, 3
when a write failed.
