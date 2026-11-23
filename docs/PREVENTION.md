# Prevention: what went wrong, what could, and reviews of the cycle

> **Not in motion yet.** The log below has no entries, and neither a review nor a look-ahead
> has run. This page holds the design and its reasons until the maintainer starts it
> (`WORKLOG.md`, "Working mode").

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

One entry per prevention line, appended when the line is made, newest last. A near miss
gets an entry too: a failure stopped by a dry run, a second look or luck, rather than by a
control. Like every tracked file, an entry never names a login, a plan or its usage.

```
## YYYY-MM-DD — <cause: a short name, reused by every entry of that cause> — <what happened>
Fix: what changed, and its control (1 to 4 above).
Near miss: for one, what stopped it (a dry run, a second look, luck); otherwise left out
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

## Looking ahead

The review acts on what has already gone wrong. A weak spot that has not failed yet leaves
nothing in the log, and is found only by looking for it: a look-ahead is the session that
does. One example, 2026-11-23: the script that removes finished worktrees listed three
running ones in its dry run. It had been patched once for that very case, but the patch
assumed a run keeps its logs inside its worktree, and these kept theirs elsewhere. Asking
"what does this script that deletes assume?" would have found it before the dry run did.

- **When:** every two or three phases, and before a phase that does something the cycle has
  never done (a first public release, a first Linux build, a new kind of tool). It never
  runs in the same session as a review.
- **What it works from:** imagined failures are always plausible, and each would become a
  rule: the pile-up this page exists to cut. So a look-ahead works from evidence too, of a
  kind that needs no incident:
  - the log's near misses;
  - a map of the cycle's steps (the dev cycle chart's), each with its guards and what a miss
    there costs. A step that is hard to undo or reaches members (a save written, a file
    deleted, a push, a release) but is guarded only by memory or one reading is a weak spot;
  - drills: a known fault planted in a copy of the work, to see whether the cycle catches it
    (a brief naming a function that does not exist, a test that cannot fail, a pointer to a
    deleted section, a help line the code contradicts). A guard that has never fired and
    misses its drill is dead;
  - what is coming: the next phases' plans, and changes around the work the cycle has never
    met (a new model, a new version of a tool it runs, an account's new limits).
- **Who:** the lead builds the map and gathers the near misses. Then a reviewer of another
  model family gets an attacker's brief, one stage of the cycle at a time (plan, brief,
  build, review, merge, gate, release, handover, and one pass over the handoffs between
  them): find the easiest way to get a wrong save or a wrong CPK to a member past this cycle.
- **Its output:** each finding ends in one of three ways:
  - a drill that shows a gap gets a fix, a script or a gate before a written rule;
  - a drill the cycle passes is recorded and changes nothing;
  - an assumption that will stop holding (an account's limit, a disk floor, a model's
    context) gets a reminder at its trigger, not a rule.

  A new written rule needs a failed drill behind it, and the review's balance holds here
  too: no more rule lines added than removed.
- **Its record:** an entry in the log naming the steps it looked at, its drills and their
  results, and its predictions: the weak spots it judged likely to fail. The next look-ahead
  checks them both ways: which predicted weak spots failed, and which incidents since came
  from steps it judged sound. That is how the sessions learn what they are good at finding.

## Guards

- A cause seen once still gets its prevention line, as now; the review acts on causes seen at
  least twice. Acting on a single incident is what prevention already does.
- A review or a look-ahead is one pass by the lead and one reviewer round, like a review of
  code, never a standing project of its own.
- The log counts causes, never who caused them: a count of a sidekick's or a reviewer's misses
  would bend the rulings toward the count.

## Starting

The first step backfills two weeks of prevention lines into the log, from the review rulings
and the lead's turn reports: it tests whether the review pays off before it becomes a standing
step. Then the first review runs, `AGENTS.md` gains this file's row in its documents table,
and the dev cycle chart (`web/dev_cycle.html`) gains a loop from its Close column back to
Plan and Brief. The first look-ahead follows the first review, starting from where the log's
incidents cluster, and from the steps where none was ever recorded.
