Unlimited Tool Works looks for new versions by itself and can install one for
you, so you do not need to download a new release by hand.

**Checking for a new version**

The program reads the list of releases on its GitHub page when it starts, unless
it checked less than 120 minutes ago, and again every 120 minutes while it stays
open. You can turn this off with `Check for updates` in the
settings, or change how often it happens with `Check interval (minutes)`. When
a check fails, because you are offline for instance, nothing is shown and the
program tries again later.

The launchers (the `.bat` files that come with the program on Windows, such
as `quick_compile.bat`), and a tool's command typed in a terminal, look for a new
version too while `Check for updates` is on, and mention one in a line after
their report. They install nothing: start the program and use the offer in the status
bar.

**The offer in the status bar**

When a newer version is out, the status bar shows `Update available` and the
version's number in blue. Nothing opens by itself: click the text, or the
`Details` button beside it, when you want to know more.

**Details**

The Details window names the new version and the one you have, and shows the release
notes, which say what the new version brings. It has four buttons:

- `Update now` downloads the new version and installs it, as described below;
- `Open release page` opens the version's page in your browser, where you can
  download it yourself;
- `Skip this version` removes the offer, and no later check offers this version
  again, although a newer one will still be offered;
- `Remind me later` closes the window and leaves the offer in the status bar.

**What Update now does**

1. It downloads the new version for your system, showing how many MB are done
   on a bar with a `Cancel` button. Cancel stops the download and brings the
   offer back, with nothing changed.
2. It checks the download against the checksums published with the release, so
   a damaged download is never installed.
3. It unpacks the new version into the program's folder. The program you are
   running moves into the `old` folder inside it, and the new version's files
   take the place of the old ones. Your settings and data are left as they are.

While the download runs, clicking outside the window or pressing Escape does not
close it, so a stray click cannot hide or stop the update. Closing the program
stops the download and waits until nothing is left half done.

**When an update fails**

If something goes wrong, `Update failed` says what happened, and that nothing
was changed, so the version you have keeps working. The cause can be the
connection, a download that does not match its checksum, or a program folder
you are not allowed to write to, which is found before anything is downloaded.
The window has three buttons:

- `Retry` starts the update again;
- `Open release page` lets you download the new version by hand instead;
- `Close` closes the window and leaves the offer in the status bar.

In the rare case where the old files could not be put back either, the message
does not say that nothing was changed, and it names the folder that holds them.
Put them back into the program's folder, or delete that folder, because no other
update starts until you do.

**Restarting**

Once the new version is in place, `Update ready` offers two buttons:

- `Restart now` closes the program and starts the new version. If a tool first
  asks about unsaved work, as it does whenever the program closes, the program
  closes or stays open as you answer, and it does not start again by itself: the
  new version runs the next time you start it;
- `Later` closes the window. The status bar then reads `Update ready`, which
  opens this window again, and the new version runs the next time you start
  the program.

**After the update**

The first time a new version starts, the status bar shows `Updated to` with its
number and a `What's new` button. The button opens the What's new chapter of
this help, which lists what changed, version by version.
