The **Appearance** screen holds a player's look, in groups from top to bottom:
**Full Player Customization** with his skin and iris **Colors** at its right,
his **Strip style**, his **Physique** with his **Motion** and the game's own
**Edit flags** beside it, and last **Presentation**. The toggle beside the
player's name picks one of three screens: **Stats** shows his fields,
**Appearance** his look, and **Gameplan** his team's fields and tactics.

**When it is locked**

A player's look can belong to his team's export: the export's `settings.toml`
and models decide it, and the Team compiler writes it into the save through the
aesthetics patch. A hand edit here would be silently overwritten the next time
the team is recompiled or patched.

That is what the **Aesthetics from exports** setting, in the Save editor's
Settings section, is for. A cup whose looks are compiled from exports turns it
on, which puts on the aesthetics lock: the screen turns read-only under a line
saying the team's export owns the aesthetics, and every other control that
writes the look, such as the Team menu's FPC items, is locked too. The setting
is off by default, for the cups whose looks live in the save, so every control
then edits and neither that line nor the switch below is drawn.

**Edit aesthetics anyway**

The **Edit aesthetics anyway** switch, under that line, unlocks the screen for
the rest of the session, for the cases the lock does not cover: a team with no
export, or a cup-night fix when recompiling is not an option. Opening another
save keeps it unlocked. Starting the tool again turns it back off.

The note under the switch warns that a later patch overwrites whatever you edit
by hand.

**Full Player Customization**

The **Full Player Customization** group, first on the screen, is where FPC
invisibility lives: a line reading from the save whether his default body is
hidden, then the **Hide default body** and **Show default body** buttons.

The reading says "The default body is currently hidden" when his boots ID in the
save is the hide ID from Settings (55 unless changed) or his skin is Custom, and
"The default body is currently shown" otherwise. A player hidden by his model's
own boots ID, such as a compile-allocated one, reads "The default body is
currently shown", because the save cannot tell his model hides him, which is why
these are two buttons and not a checkbox.

- **Hide default body** writes long sleeves, a tucked shirt and short socks, and
  a boots or gloves ID of 0 takes the hide ID from Settings (55 and 11 unless
  changed, while a model's own ID is kept).
- **Show default body** writes short sleeves, an untucked shirt and standard
  socks, and a boots or gloves ID that holds the hide ID goes back to 0.

On PES 2015 to 2017 both also turn a Custom skin into Light. Changing the hide
ID in Settings rewrites no save, so a player hidden with the old ID then reads "The
default body is currently shown", and Show default body leaves his ID alone (it
is no longer the hide ID).

**Settings that interfere with a hidden body**

Under the reading the group lists the strip settings that would interfere with a
hidden body (inners or undershorts that break the hiding, taping that still
shows) and **Outfielder gloves** set to Worn in winter with a gloves ID of 0,
which puts winter gloves on the player in winter conditions. They are reported,
never changed for you. The inners, undershorts and
taping follow the save's own reading, so they are listed only for a player the
save calls hidden.

For a player hidden by his model's own boots ID (a compile-allocated one such
as 627), whom the reading calls shown, the inners, undershorts and taping are
not listed, though the game hides his body all the same. The Team compiler's check knows the player folder's
FPC marker, so it reports them for a compiled team.

**Randomize**

**Randomize**, in the Motion group, draws new values for the hunching, arm
movement, kick and celebration motions, each within the range the game's Edit
mode offers, and the dribbling motion is left as it is. One Undo restores them
all.

**Presentation**

The **Presentation** group, last on the screen, holds **Star**, which sets how
likely the player is to show up as the team's featured player in the pre-match
screen, and **Playing attitude**, which only affects cutscenes. Neither changes
how he plays, but both belong to his team's export with the rest of his look,
so the lock covers them too.
