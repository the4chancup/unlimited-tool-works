The collapsed **Appearance** section at the bottom of the player view holds his
look: the game's own edit flags, his physique, his strip style, his motions, and
his skin and iris colours.

**Cutscenes**

The **Cutscenes** group comes first and stays editable when the rest is locked.
Star and playing attitude only change how the player appears in cutscenes, so
they are not part of the look the team's export owns.

**When it is locked**

A player's look can belong to his team's export: the export's `settings.toml`
and models decide it, and the Team compiler writes it into the save through the
aesthetics patch. A hand edit here would be silently overwritten the next time
the team is recompiled or patched.

That is what **Aesthetics from exports** in the Save editor's Settings section
is for: with it on, the section and every other aesthetics path open read-only
behind the ownership line, for a cup whose looks are compiled from exports. It
is off by default, for the cups whose looks live in the save, so every control
then edits and no ownership line or switch is drawn.

**Edit aesthetics anyway**

**Edit aesthetics anyway** draws under the setting and unlocks the section for
the rest of the session, for the cases the rule does not cover: a team with no
export, or a cup-night fix when recompiling is not an option. Opening another
save keeps it unlocked. Starting the tool again turns it back off.

The note under the switch warns that a later patch overwrites whatever you edit
by hand.

**Randomize**

**Randomize** draws new values for the hunching, arm movement, kick and
celebration motions, each within the range the game's Edit mode offers, and the
dribbling motion is left as it is. One Undo restores them all.

**Full Player Customization**

The **Full Player Customization** group, last in the section, is where FPC
invisibility lives: the save's own reading of the player, then **Hide default
body** and **Show default body**.

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

On PES 2015 to 2017 both also turn a Custom skin into Light. Changing the
setting rewrites no save, so a player hidden with the old ID then reads "The
default body is currently shown", and Show default body leaves his ID alone (it
is no longer the hide ID).

**Settings that interfere with a hidden body**

Under the reading the group lists the strip settings that would interfere with a
hidden body (inners or undershorts that break the hiding, taping that still
shows) and the gloves checkbox with ID 0, which gives winter gloves to any
player. They are reported, never changed for you. The inners, undershorts and
taping follow the save's own reading, so they are listed only for a player the
save calls hidden.

A player hidden by his model's own boots ID (a compile-allocated one such
as 627) reads "The default body is currently shown", and they are not listed for
him, though the game hides his body all the same. The Team compiler's check
knows the folder's FPC marker, so it reports them for a compiled team.
