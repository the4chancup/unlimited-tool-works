# Prevention: what went wrong, and a review of the cycle that let it

> **Not in motion yet.** The log below has no entries and the first review has not run. This
> page holds the design and its reasons until the maintainer starts it (`WORKLOG.md`,
> "Working mode").

## Why

Every accepted review concern and every snag (a tool that failed, a retry, a step done twice)
gets a prevention line (`AGENTS.md` "Prevention"): a brief rule, a skill line, a gate or a
review sweep, written while looking at that one incident. Nothing looks across them, so a
cause that keeps coming back gets a new line each time, and the lines pile up: every session
and every brief pays to read them, whether they work or not.

One day's work, 2026-11-23, showed both:

- three slices in a row came back with test setup copied instead of shared, although the
  lead's briefing skill (`~/.claude/skills/helium`) had gained a line against it after the first and again after the
  second;
- two briefs left out what changes the state they relied on (a settings comparison that
  re-checked the whole grid at every tool switch; an output mode left runnable after a switch
  to a PES version that cannot run it), and got two separate skill lines for one cause;
- two tests asserted on a moment rather than an outcome (a lock taken again right after it
  was freed; a check expected to be running just after it started, though it can end at
  once);
- two generated pages, the dev cycle chart and the crate map, had fallen behind what they
  draw, one of them by eight days.

## Controls, strongest first

1. Remove the cause: the step that could go wrong no longer exists.
2. A script or gate that refuses: `commit.py`'s message check, the mutation runs' disk floor.
3. A written rule: a line in a skill, `RUNBOOK.md`, `CONTRIBUTING.md` or the lead's rules
   (`RULES.md` in its memory folder).
4. A review that catches it each time.

Most prevention lines are of the third kind, and the copied test setup shows their limit: a
rule a delegate can miss does not stop a cause that recurs. The review exists to move such a
cause up this list.

## The log

One entry per prevention line, appended when the line is made, newest last. Like every
tracked file, an entry never names a login, a plan or its usage.

```
## YYYY-MM-DD — <cause: a short name, reused by every entry of that cause> — <what happened>
Fix: what changed, and its control (1 to 4 above).
Recurred: none (the review writes the date and what came back, when the cause returns
after this fix)
```

## The review

- **When:** after about 20 new entries, or at a phase's close, whichever comes first. How
  many incidents there are says more about what needs reviewing than how long it has been.
- **Who:** the lead first groups the entries by cause and counts the causes that came back
  after a fix. Then a reviewer of another model family reads the log, the lead's rules, the
  briefing skill and `RUNBOOK.md`: the lead wrote most of those rules, so it is the wrong
  one to judge them alone.
- **What it asks:**
  - which causes recur despite their fix (each gets a stronger control);
  - which rules duplicate or contradict each other, or never fired (merged or deleted);
  - where a handoff loses what the next step needs (lead to sidekick, lead to reviewer, one
    session to the next).
- **Its output:** a short list of changes, each naming the entries it answers. It removes at
  least as many rule lines as it adds, since a review that only adds lines feeds the cost it
  exists to cut. A change to how the maintainer works goes first to the maintainer, as a
  card on `maintainer/proposals.html`.
- **Its record:** an entry in the log naming the entries it covered and what changed.

## Guards

- A cause seen once still gets its prevention line, as now; the review acts on causes seen at
  least twice. Acting on a single incident is what prevention already does.
- A review is one pass by the lead and one reviewer round, like a review of code, never a
  standing project of its own.
- The log counts causes, never who caused them: a count of a sidekick's or a reviewer's misses
  would bend the rulings toward the count.

## Starting

The first step backfills two weeks of prevention lines into the log, from the review rulings
and the lead's turn reports: it tests whether the review pays off before it becomes a standing
step. Then the first review runs, `AGENTS.md` gains this file's row in its documents table,
and the dev cycle chart (`web/dev_cycle.html`) gains a loop from its Close column back to
Plan and Brief.
