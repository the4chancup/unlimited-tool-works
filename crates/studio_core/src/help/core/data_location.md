When a `data/` folder holding a `settings.toml` file already sits beside
`UTW.exe`, as it does in a download that comes with its settings, the first
run asks nothing, and your settings and data stay in that folder. When there
is no such folder but your user folder (below) already holds a
`settings.toml`, from an earlier copy of the program for instance, the first
run asks nothing either and uses that one.

Only when neither place holds a `settings.toml` does the first run ask where
your settings and data should live:

- `Next to the program` puts them in a `data/` folder beside `UTW.exe`, which
  is fully portable, because the folder carries the program and everything it
  keeps, and deleting it removes both;
- `In my user folder` puts them in `%APPDATA%\unlimited-tool-works` on Windows
  or `~/.config/unlimited-tool-works` on Linux, so the data survives deleting
  or replacing the program folder.
