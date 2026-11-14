The **Player** menu works on the selected player:

- **Set all stats…** gives every ability stat the same value, 40 to 99, and the
  dialog's value starts at his Attacking Prowess (weak foot, form and injury are
  not ability stats and stay as they are).
- **Bump all stats…** (Ctrl+B) adds the same amount, -10 to +10, to every
  ability stat, each kept within 40 to 99, and the dialog remembers the last
  amount used in this window.
- **Hide default body (Enable FPC)** or **Show default body (Disable FPC)**
  (Ctrl+I): the entry's label names what the press will do, following the
  Appearance section's FPC reading: it shows the default body of a player
  who reads "The default body is currently hidden" and hides any other's,
  the same write as the section's Show default body and Hide default body.
- **Copy/swap stats…** opens a window that copies one player's stats onto another,
  or trades them, and stays open for the next player.

Every operation is one undoable step. Set all stats and Bump all stats write the
same fields the player view's abilities and skills part edits, so they set no
edit flag. Copy and Swap stats move the gameplay edit flags with the stats.

**Hide and Show default body**

A player hidden by his model's own boots ID reads "The default body is currently
shown", so for him the entry hides him again; to show his default body, use the
section's Show default body instead. The entry stays disabled while the aesthetics lock is on (see the
Appearance section).

**Copy and Swap stats**

The window has two rows, Source and Destination, each picked by team, then by
roster slot; both start on the selected player. It stays open over the editor,
and selecting a row in the player list makes that player the Destination, so
one player's stats go onto several others with a click on each and **Copy**.
**Swap** trades the two players' stats instead. A line in the window says what
the last Copy or Swap did.

Height, weight, every
ability, form, injury, weak foot, positions, playing style, skills and COM
styles, stronger foot and hand, and the gameplay edit
flags move with the stats. Names, nationality, age, every motion, the star
and playing attitude, and every appearance field stay as they are.

**Generating a player folder**

The menu's **Generate settings.toml…** and **Generate in-game face…** write the selected
player's look to a player folder of your choice, the files a team moving to
compile-time aesthetics is built from. `settings.toml` holds everything but the
in-game face: the names, the stock boots and gloves IDs, skin color, physique,
strip, motions, and the star and playing attitude. `ingame_face.toml` holds
the in-game face itself: iris color, face and hair.

The save cannot say which players wear the in-game face, so you choose per
player whether to generate it. Neither item writes the save.

When the folder already holds a `settings.toml`, its comments stay while the
values become the player's. `ingame_face.toml` is written whole.
