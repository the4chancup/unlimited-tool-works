The **Player** menu works on the selected player:

- **Set all stats…** gives every ability stat the same value, 40 to 99, and the
  dialog's value starts at his Attacking Prowess (weak foot, form and injury are
  not ability stats and stay as they are).
- **Bump all stats…** (Ctrl+B) adds the same amount, -10 to +10, to every
  ability stat, clamped to 40 to 99, and the dialog remembers the last amount
  used in this window.
- **Toggle FPC settings** (Ctrl+I) follows the Appearance section's FPC reading:
  it shows a player who reads "The default body is currently hidden" and hides
  any other, the same write as the section's Show default body and Hide default
  body.
- **Copy stats from…** copies another player's stats onto the selected one.
- **Swap stats with…** trades them.

Every operation is one undoable step. Set all stats and Bump all stats write the
same fields the player view's abilities and skills part edits, so they set no
edit flag. Copy and Swap stats move the gameplay edit flags with the stats.

**Toggle FPC settings**

A player hidden by his model's own boots ID reads "The default body is currently
shown", so for him the toggle hides him again. Use the section's Show default
body on him. The toggle stays disabled while the aesthetics lock is on (see the
Appearance section).

**Copy and Swap stats**

The other player is picked by team, then by roster slot. Height, weight, every
ability, form, injury, weak foot, positions, playing style, skills and COM
styles, stronger foot and hand, star, playing attitude, the free-kick motion and
the gameplay edit flags move with the stats. Names, nationality, age, the other
motions and every appearance field stay as they are.

**Generating a player folder**

**Generate settings.toml…** and **Generate in-game face…** write the selected
player's look to a player folder of your choice, the files a team moving to
compile-time aesthetics is built from. `settings.toml` holds everything but the
in-game face: the names, the stock boots and gloves IDs, skin colour, physique,
strip and motions. `ingame_face.toml` holds the in-game face itself: iris
colour, face and hair.

The save cannot say which players wear the in-game face, so you choose per
player whether to generate it, and neither writes the save.

When the folder already holds a `settings.toml`, its comments stay while the
values become the player's. `ingame_face.toml` is written whole.
