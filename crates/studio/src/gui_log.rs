//! The window's diagnostic sink: `utw.log` in the data location, the previous run's
//! renamed to `utw.old.log` at start, every line flushed as it is written and the
//! user's home folder written as `~`, because the file is what a member posts with a bug
//! report (ST-RUN-07). A panic is written to it with a
//! backtrace of module bases and offsets by the panic hook, and a panic that ends the
//! window raises a dialog naming the file before the process exits with the code an
//! unguarded panic would (ST-RUN-08). A session ends with a closing line, so an old log
//! ending without one means a session that crashed or was ended, told to the member at
//! the next start before the window opens (ST-RUN-09). The CLI and help modes keep
//! `env_logger`: a log file a member is asked to post is a GUI-mode need.

use std::any::Any;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Mutex, MutexGuard, PoisonError};

use directories::BaseDirs;
use log::{Level, LevelFilter, Metadata, Record};
use studio_core::version_label;

/// `utw.log`'s name in the data location.
pub(crate) const LOG_FILE_NAME: &str = "utw.log";
/// What the previous run's log is renamed to, so a crash followed by a relaunch keeps
/// the crash's log.
const OLD_LOG_FILE_NAME: &str = "utw.old.log";

/// The session's last log line on a normal close and on a crash that was shown to the
/// member (ST-RUN-09): the next start's `utw.old.log` ending with neither means the
/// session crashed in a way the hook cannot see or was ended by Windows, and a dialog
/// says so before the window opens.
const CLOSING_LINE: &str = "the window closed normally";
/// The session's last log line on a panic whose dialog was shown (ST-RUN-09): reported
/// already, so the next start does not ask for the log again.
const CRASH_REPORTED_LINE: &str = "the crash above was shown to the member";

/// `line` with the user's home folder written as `~`, because findings carry absolute
/// paths and the log is posted where the cup's members can read it (ST-RUN-07). No home
/// known (`None`) and an empty one both leave the line as it is. On Windows, where paths
/// ignore letter case and take either slash, the home folder is matched however it is
/// written (a PES folder typed `c:/users/...`).
pub(crate) fn redact_home(line: &str, home: Option<&str>) -> String {
    match home {
        Some(home) if !home.is_empty() && cfg!(windows) => redact_any_spelling(line, home),
        Some(home) if !home.is_empty() => line.replace(home, "~"),
        _ => line.to_string(),
    }
}

/// `line` with every run of characters that spells `home` in any letter case, with `/`
/// and `\` alike, written as `~`; the rest of the line as written.
fn redact_any_spelling(line: &str, home: &str) -> String {
    let same = |a: char, b: char| {
        matches!((a, b), ('/' | '\\', '/' | '\\')) || a.to_lowercase().eq(b.to_lowercase())
    };
    // The byte length of `rest`'s start when it spells `home`.
    let spelled_len = |rest: &str| {
        let mut chars = rest.chars();
        let mut len = 0;
        for wanted in home.chars() {
            let found = chars.next().filter(|found| same(wanted, *found))?;
            len += found.len_utf8();
        }
        Some(len)
    };
    let mut redacted = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(next) = rest.chars().next() {
        let taken = match spelled_len(rest) {
            Some(len) => {
                redacted.push('~');
                len
            }
            None => {
                redacted.push(next);
                next.len_utf8()
            }
        };
        rest = &rest[taken..];
    }
    redacted
}

/// One log line as `FileLog::log` writes it (`"INFO  session: …\n"`), factored out so
/// the crash monitor in its own process appends a line in the same shape (ST-RUN-10).
pub(crate) fn log_line(level: Level, target: &str, text: &dyn std::fmt::Display) -> String {
    format!("{:<5} {}: {}\n", level, target, text)
}

/// How the previous run's log ended (ST-RUN-09, ST-RUN-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LastSession {
    /// No previous log, or it ended with a closing line: nothing to tell the member.
    Closed,
    /// It ended without one: a crash nothing caught, or a session Windows ended.
    Unclean,
    /// It ended with the monitor's line: a crash whose dump was written beside the log.
    Dumped,
}

/// How the session `log` holds ended (ST-RUN-09, ST-RUN-10). The lines read
/// `INFO  session: <line>` or `ERROR crash_dump: <line>`, so a marker is what a line
/// *ends* with. A closing line counts only last, and wins: a second window's monitor
/// may append into this log (the second start renamed the first window's log away), and
/// this session still closed. Otherwise the monitor's line counts anywhere, not only
/// last: the crashed thread waits for the monitor's reply while the window's other
/// threads run on and may log after it. A log with no line at all (a session that ended
/// before its first) is unclean.
fn last_session(log: &str) -> LastSession {
    let last = log.lines().rev().find(|line| !line.trim().is_empty());
    if last.is_some_and(|last| last.ends_with(CLOSING_LINE) || last.ends_with(CRASH_REPORTED_LINE))
    {
        return LastSession::Closed;
    }
    if log
        .lines()
        .any(|line| line.ends_with(crate::crash_dump::DUMP_WRITTEN_LINE))
    {
        LastSession::Dumped
    } else {
        LastSession::Unclean
    }
}

/// What `open_log` opened: `utw.log`'s file, and how the previous run's log it
/// rotated away ended (ST-RUN-09, ST-RUN-10).
pub(crate) struct OpenedLog {
    file: File,
    pub(crate) last_session: LastSession,
}

/// Creates `data_dir/utw.log` with `first_line` as its first line, the previous run's
/// renamed to `utw.old.log` first: `fs::rename` replaces the older file on Windows
/// and Linux alike, so a crash followed by a relaunch keeps the crash's log. The old
/// log is read before the rename: its lines are what tell an ended session from a clean
/// one (`last_session`; ST-RUN-09, ST-RUN-10).
pub(crate) fn open_log(data_dir: &Path, first_line: &str) -> io::Result<OpenedLog> {
    let path = data_dir.join(LOG_FILE_NAME);
    let last_session = if path.exists() {
        let old = fs::read(&path)?;
        fs::rename(&path, data_dir.join(OLD_LOG_FILE_NAME))?;
        last_session(&String::from_utf8_lossy(&old))
    } else {
        LastSession::Closed
    };
    // Append, never write-at-offset: the crash monitor appends its own line to this
    // file from another process, and a window thread still writing at the offset it
    // held would overwrite it (ST-RUN-10). The old log was renamed away, so the file
    // starts empty either way.
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(file, "{first_line}")?;
    Ok(OpenedLog { file, last_session })
}

/// The `log::Log` writing `utw.log`, at most `Info`, since a double-click passes no `-v`
/// or `RUST_LOG`. A struct, not a closure, so the tests can drive the
/// `log` method — a process holds one logger and `main`'s tests already install the
/// CLI's.
struct FileLog {
    file: Mutex<File>,
    home: Option<String>,
}

impl FileLog {
    /// The lock survives a panic mid-write: a panic's own line must not be lost to the
    /// poisoning it caused.
    fn locked(&self) -> MutexGuard<'_, File> {
        self.file.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl log::Log for FileLog {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = redact_home(
            &log_line(record.level(), record.target(), &record.args()),
            self.home.as_deref(),
        );
        let mut file = self.locked();
        // Each line is flushed at once, so a crash keeps everything before it. The log
        // is where failures would be reported, so its own failed write has nowhere to
        // report itself: it is dropped.
        #[expect(
            clippy::let_underscore_must_use,
            reason = "a failed write to the log has nowhere to be reported"
        )]
        let _ = file.write_all(line.as_bytes()).and_then(|()| file.flush());
    }

    fn flush(&self) {
        // `FileLog::log` flushes every line as it writes it, so this has nothing of its own to do.
        #[expect(
            clippy::let_underscore_must_use,
            reason = "a failed flush of the log has nowhere to be reported"
        )]
        let _ = self.locked().flush();
    }
}

/// What `install_gui_logger` reports back: `utw.log`'s path, and how the previous
/// run's log ended, the signal for the unclean-shutdown dialog the next start shows
/// before the window opens (ST-RUN-09).
pub(crate) struct InstalledLog {
    /// `utw.log`'s path in the data location.
    pub(crate) path: PathBuf,
    /// How the previous run's `utw.log` ended (ST-RUN-09, ST-RUN-10).
    pub(crate) last_session: LastSession,
}

impl InstalledLog {
    /// `utw.old.log`'s path: the previous run's `utw.log` as `open_log` renamed it.
    pub(crate) fn old_path(&self) -> PathBuf {
        self.path.with_file_name(OLD_LOG_FILE_NAME)
    }

    /// `utw.dmp`'s path beside the log when the previous session's monitor said it
    /// wrote one and the file is still there (ST-RUN-10), else `None`.
    pub(crate) fn dump_path(&self) -> Option<PathBuf> {
        if self.last_session != LastSession::Dumped {
            return None;
        }
        let path = self.path.with_file_name(crate::crash_dump::DUMP_FILE_NAME);
        path.is_file().then_some(path)
    }
}

/// Installs `utw.log` as the `log` sink at level `Info` and returns what it opened.
/// `None` means no log this run: no data location yet (a first run, whose dialog has
/// not chosen one — a log written elsewhere would leave a folder behind that the
/// portable choice promised not to), or the file cannot be opened (a read-only location
/// the status bar already reports) (ST-RUN-07).
pub(crate) fn install_gui_logger(data_dir: Option<&Path>) -> Option<InstalledLog> {
    let data_dir = data_dir?;
    let first_line = format!("{} {}", studio_core::app_name!(), version_label());
    let opened = open_log(data_dir, &first_line).ok()?;
    let home = BaseDirs::new().map(|dirs| dirs.home_dir().display().to_string());
    log::set_boxed_logger(Box::new(FileLog {
        file: Mutex::new(opened.file),
        home,
    }))
    .ok()?;
    log::set_max_level(LevelFilter::Info);
    Some(InstalledLog {
        path: data_dir.join(LOG_FILE_NAME),
        last_session: opened.last_session,
    })
}

/// The hook that writes every panic to the log — the `PanicHookInfo`'s `Display` (its
/// message and location), then a backtrace — chained before the hook that was installed
/// when it was made, so a terminal still prints the panic as before.
fn panic_hook(logger: &'static dyn log::Log) -> Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync> {
    let previous = panic::take_hook();
    Box::new(move |info| {
        logger.log(
            &Record::builder()
                .level(Level::Error)
                .target("panic")
                .args(format_args!("{info}\n{}", backtrace_lines()))
                .build(),
        );
        logger.flush();
        previous(info);
    })
}

/// Installs the panic hook over the `log` sink installed by `install_gui_logger`; with
/// none (a first run) `log::logger()` is a nop and only the chained hook still prints.
pub(crate) fn install_panic_hook() {
    panic::set_hook(panic_hook(log::logger()));
}

/// One backtrace frame as `{index}: 0x<base>+0x<offset>` (the frame's module base and
/// the offset into it), not a name: a release exe names its frames only with its `.pdb`
/// beside it, which the bundle does not ship, so the names are resolved later by
/// `scripts/symbolize_log.py` against the release's kept symbols. Frame 0 is always
/// inside the exe (the `backtrace` crate is
/// linked into it), which is how the script tells the exe's module from the system's.
/// A frame no module claims has no base and reads `{index}: 0x<ip>`; it stays unnamed.
fn frame_line(index: usize, ip: usize, base: Option<usize>) -> String {
    match base {
        Some(base) => format!("{index:>3}: 0x{base:x}+0x{:x}", ip - base),
        None => format!("{index:>3}: 0x{ip:x}"),
    }
}

/// The running thread's frames, one `frame_line` per line. Nothing is resolved here:
/// resolving needs the `.pdb` the bundle does not ship, and it costs time while the
/// process is dying.
fn backtrace_lines() -> String {
    backtrace::Backtrace::new_unresolved()
        .frames()
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            frame_line(
                index,
                frame.ip().addr(),
                frame.module_base_address().map(|base| base.addr()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The crash dialog's text, written for a member: what happened, and the file — or the
/// message, when no log exists — to post where problems are reported. No code
/// vocabulary reaches it (CONTRIBUTING.md "Code style").
fn crash_text(message: &str, log: Option<&Path>) -> String {
    let what = format!(
        "{} stopped because of an error in its own code: {message}",
        studio_core::app_name!()
    );
    match log {
        Some(path) => format!(
            "{what}\n\nPlease post this file where you report problems: {}. It holds what the program was doing, and your home folder is written in it as ~.",
            path.display()
        ),
        None => format!("{what}\n\nPlease post this message where you report problems."),
    }
}

/// What a panic's payload says: the `&str` or `String` `panic!` carried, or a fallback
/// for a payload of anything else.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "an error without a message".to_string()
}

/// The next start's dialog when the previous session left no closing line (ST-RUN-09):
/// the session may have crashed in a way the hook cannot see, or Windows may have ended
/// it, and the wording stays neutral between the two. `old_log` is `utw.old.log`'s
/// path; `dump` is `utw.dmp`'s when a crashed session's monitor wrote one
/// (ST-RUN-10), asked for beside the log; the dump may still show the member's user
/// name (its module list holds the exe's full path), which the log writes as `~`.
pub(crate) fn unclean_text(old_log: &Path, dump: Option<&Path>) -> String {
    match dump {
        Some(dump) => format!(
            "The last session did not close normally. If it crashed, please post these two files where you report problems: {} and {}. Your home folder is written in the log as ~, but {} may still show your user name.",
            old_log.display(),
            dump.display(),
            crate::crash_dump::DUMP_FILE_NAME
        ),
        None => format!(
            "The last session did not close normally. If it crashed, please post this file where you report problems: {}. Your home folder is written in it as ~.",
            old_log.display()
        ),
    }
}

/// The notice the window's start shows before it opens (ST-RUN-09, ST-RUN-10):
/// `unclean_text` for `installed`'s previous session when it did not close, naming the
/// dump when the monitor wrote one; `None` when it closed.
pub(crate) fn unclean_notice(installed: &InstalledLog) -> Option<String> {
    if installed.last_session == LastSession::Closed {
        return None;
    }
    Some(unclean_text(
        &installed.old_path(),
        installed.dump_path().as_deref(),
    ))
}

/// One `session` line through `logger` at `Info`, the level `FileLog` keeps.
fn session_line(logger: &dyn log::Log, line: &'static str) {
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .target("session")
            .args(format_args!("{line}"))
            .build(),
    );
}

/// Runs the window inside a panic guard: a clean return is the window's own code, while
/// a panic is told to the member through `show` and exits 101 — the code an unguarded
/// panic exits with, so launchers see the same code as before (ST-RUN-08). A panic on
/// another thread is logged by the hook but raises no dialog here: the hook cannot tell
/// a panic that will be recovered from one that ends the process. `logger` gets the
/// session's closing line (ST-RUN-09): a clean return writes `CLOSING_LINE` (the
/// window's own error exit included, that failure already told) and a reported panic
/// writes `CRASH_REPORTED_LINE` once `show` has run.
pub(crate) fn guard_gui(
    run: impl FnOnce() -> ExitCode,
    log: Option<&Path>,
    show: impl FnOnce(&str),
    logger: &dyn log::Log,
) -> ExitCode {
    let code = match panic::catch_unwind(AssertUnwindSafe(run)) {
        Ok(code) => {
            session_line(logger, CLOSING_LINE);
            code
        }
        Err(payload) => {
            show(&crash_text(&panic_message(payload.as_ref()), log));
            session_line(logger, CRASH_REPORTED_LINE);
            ExitCode::from(101)
        }
    };
    logger.flush();
    code
}

/// The blocking dialog raised when a double-clicked window cannot print: `Error` for a
/// startup error, a window that could not open or the panic that ended it; `Info` for
/// the unclean-shutdown notice, whose session may have been ended by Windows rather
/// than crashed, so its wording stays neutral. Titled with the suite's
/// name, one OK button.
pub(crate) fn show_dialog(level: rfd::MessageLevel, text: &str) {
    rfd::MessageDialog::new()
        .set_level(level)
        .set_title(studio_core::app_name!())
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use log::Log;

    use super::*;
    use crate::crash_dump::{DUMP_FILE_NAME, DUMP_WRITTEN_LINE};
    use crate::test_dir::TestDir;

    // ST-RUN-07
    #[test]
    fn the_previous_runs_log_is_kept_as_the_old_log() {
        let dir = TestDir::new("rotate");
        fs::write(dir.path().join(LOG_FILE_NAME), "second run\n").unwrap();
        fs::write(dir.path().join(OLD_LOG_FILE_NAME), "first run\n").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(OLD_LOG_FILE_NAME)).unwrap(),
            "first run\n"
        );
        open_log(dir.path(), "Unlimited Tool Works v0.0.1 pre-alpha").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(OLD_LOG_FILE_NAME)).unwrap(),
            "second run\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap(),
            "Unlimited Tool Works v0.0.1 pre-alpha\n"
        );
    }

    // ST-RUN-07
    #[test]
    fn a_first_run_leaves_no_old_log() {
        let dir = TestDir::new("fresh");
        open_log(dir.path(), "Unlimited Tool Works v0.0.1 pre-alpha").unwrap();
        assert!(!dir.path().join(OLD_LOG_FILE_NAME).exists());
    }

    // ST-RUN-07
    #[test]
    fn a_logged_line_writes_the_home_folder_as_a_tilde() {
        let dir = TestDir::new("redact");
        let path = dir.path().join(LOG_FILE_NAME);
        let logger = FileLog {
            file: Mutex::new(File::create(&path).unwrap()),
            home: Some("C:\\Users\\member".to_string()),
        };
        logger.log(
            &Record::builder()
                .level(Level::Info)
                .target("studio::gui_log")
                .args(format_args!(
                    "opened C:\\Users\\member\\Documents\\KONAMI\\a.bin and C:\\Users\\member\\b"
                ))
                .build(),
        );
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(
            written,
            "INFO  studio::gui_log: opened ~\\Documents\\KONAMI\\a.bin and ~\\b\n"
        );
        logger.log(
            &Record::builder()
                .level(Level::Debug)
                .target("studio::gui_log")
                .args(format_args!("hidden"))
                .build(),
        );
        assert!(!logger.enabled(&Metadata::builder().level(Level::Debug).build()));
        assert_eq!(fs::read_to_string(&path).unwrap(), written);
    }

    /// Windows paths ignore letter case and take either slash, so the home
    /// folder written another way is redacted too, the rest of the line as
    /// written.
    #[cfg(windows)]
    // ST-RUN-07
    #[test]
    fn redact_home_matches_any_spelling_of_the_home_folder_on_windows() {
        let home = Some("C:\\Users\\Alice");
        assert_eq!(
            redact_home("PES folder C:/Users/Alice/Games/PES2017 is missing", home),
            "PES folder ~/Games/PES2017 is missing"
        );
        assert_eq!(
            redact_home("opened c:\\users\\alice\\x", home),
            "opened ~\\x"
        );
    }

    #[test]
    fn redact_home_without_a_home_leaves_the_line_as_it_is() {
        let line = "opened C:\\Users\\member\\a.bin";
        assert_eq!(redact_home(line, None), line);
        assert_eq!(redact_home(line, Some("")), line);
    }

    /// The parts of a `frame_line` (`N: 0x<base>+0x<offset>` or `N: 0x<ip>`), for the
    /// backtrace tests' format checks; `None` for any other line.
    fn frame_parts(line: &str) -> Option<(usize, &str, Option<&str>)> {
        let (index, address) = line.trim().split_once(": 0x")?;
        let index = index.parse().ok()?;
        match address.split_once("+0x") {
            Some((base, offset)) if is_hex(base) && is_hex(offset) => {
                Some((index, base, Some(offset)))
            }
            None if is_hex(address) => Some((index, address, None)),
            _ => None,
        }
    }

    /// A nonempty string of nothing but ASCII hex digits.
    fn is_hex(text: &str) -> bool {
        !text.is_empty() && text.bytes().all(|b| b.is_ascii_hexdigit())
    }

    /// The logger `panic_hook` is tested over: a `&'static` test double collecting each
    /// record's level, target and text plus whether `flush` ran.
    #[derive(Default)]
    struct CollectLog {
        records: Mutex<Vec<String>>,
        flushed: AtomicBool,
    }

    impl log::Log for CollectLog {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &Record<'_>) {
            self.records.lock().unwrap().push(format!(
                "{:?}|{}|{}",
                record.level(),
                record.target(),
                record.args()
            ));
        }

        fn flush(&self) {
            self.flushed.store(true, Ordering::Relaxed);
        }
    }

    /// A `log::Log` pushing each record (`target|text`) and each `flush` into one list
    /// the test shares with its `show` closure, so the session lines' order against the
    /// dialog's is what is asserted (ST-RUN-09).
    struct OrderedLog(Arc<Mutex<Vec<String>>>);

    impl log::Log for OrderedLog {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &Record<'_>) {
            self.0
                .lock()
                .unwrap()
                .push(format!("{}|{}", record.target(), record.args()));
        }

        fn flush(&self) {
            self.0.lock().unwrap().push("flush".to_string());
        }
    }

    #[test]
    fn the_panic_hook_writes_the_panic_and_its_backtrace_to_the_log() {
        let logger: &'static CollectLog = Box::leak(Box::default());
        // The harness's hook is parked aside and restored before any assertion, so a
        // failing assertion panics through it; meanwhile the hook under test chains the
        // default hook `take_hook` registered in its place.
        let saved = panic::take_hook();
        panic::set_hook(panic_hook(logger));
        let result = panic::catch_unwind(|| panic!("probe 7"));
        panic::set_hook(saved);
        assert!(result.is_err());
        let records = logger.records.lock().unwrap();
        let record = records
            .iter()
            .find(|record| record.contains("probe 7"))
            .unwrap_or_else(|| panic!("no panic record; got {records:?}"));
        assert!(record.starts_with("Error|panic|"), "{record}");
        let frames: Vec<(usize, &str, Option<&str>)> =
            record.lines().filter_map(frame_parts).collect();
        assert!(!frames.is_empty(), "no frame lines in {record}");
        // On Windows every frame's module is known, so the exe's frames (frame 0 among
        // them) read `0x<base>+0x<offset>`; elsewhere a bare `0x<ip>` is enough to count.
        if cfg!(windows) {
            assert!(
                frames
                    .iter()
                    .any(|(index, _, offset)| *index == 0 && offset.is_some()),
                "no `0: 0x<base>+0x<offset>` frame in {record}"
            );
        }
        assert!(logger.flushed.load(Ordering::Relaxed));
    }

    #[test]
    fn frame_line_writes_the_offset_into_the_module_or_the_bare_address() {
        assert_eq!(
            frame_line(3, 0x7ff6_a123_59c4, Some(0x7ff6_a123_0000)),
            "  3: 0x7ff6a1230000+0x59c4"
        );
        assert_eq!(frame_line(3, 0x7ff6_a123_59c4, None), "  3: 0x7ff6a12359c4");
    }

    // ST-RUN-09: a clean run (the window's own error exit included) leaves the closing
    // line as the log's last, so the next start shows no unclean-shutdown dialog.
    #[test]
    fn a_clean_run_returns_its_own_code_and_shows_no_dialog() {
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let dialog = events.clone();
        let code = guard_gui(
            || ExitCode::from(2),
            None,
            move |text| dialog.lock().unwrap().push(format!("dialog|{text}")),
            &OrderedLog(events.clone()),
        );
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
        assert_eq!(
            *events.lock().unwrap(),
            vec![format!("session|{CLOSING_LINE}"), "flush".to_string()]
        );
    }

    // ST-RUN-09: the reported-crash line lands after the dialog, so a crash the member
    // was told about does not raise the next start's dialog.
    #[test]
    fn a_panic_shows_the_dialog_text_and_exits_with_101() {
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let dialog = events.clone();
        let code = guard_gui(
            || panic!("probe 8"),
            Some(Path::new("C:\\data\\utw.log")),
            move |text| dialog.lock().unwrap().push(format!("dialog|{text}")),
            &OrderedLog(events.clone()),
        );
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(101)));
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 3, "{events:?}");
        assert!(events[0].starts_with("dialog|"), "{events:?}");
        assert!(events[0].contains("probe 8"), "{}", events[0]);
        assert!(events[0].contains("utw.log"), "{}", events[0]);
        assert_eq!(events[1], format!("session|{CRASH_REPORTED_LINE}"));
        assert_eq!(events[2], "flush");
    }

    #[test]
    fn crash_text_without_a_log_path_asks_for_the_message() {
        let text = crash_text("the window froze", None);
        assert!(text.contains("the window froze"), "{text}");
        assert!(
            text.contains("Please post this message where you report problems."),
            "{text}"
        );
    }

    // ST-RUN-09, ST-RUN-10
    #[test]
    fn last_session_reads_the_logs_last_non_empty_line() {
        let version = "Unlimited Tool Works v0.0.1 pre-alpha";
        for marker in [CLOSING_LINE, CRASH_REPORTED_LINE] {
            let clean = format!("{version}\nINFO  session: {marker}");
            assert_eq!(last_session(&clean), LastSession::Closed, "{clean}");
            assert_eq!(
                last_session(&format!("{clean}\n")),
                LastSession::Closed,
                "{clean}+newline"
            );
        }
        // A log the session left empty (it ended before its first line) is unclean too.
        assert_eq!(last_session(""), LastSession::Unclean);
        assert_eq!(last_session(&format!("{version}\n")), LastSession::Unclean);

        let dumped = format!(
            "{version}\n{}",
            log_line(Level::Error, "crash_dump", &DUMP_WRITTEN_LINE)
        );
        assert_eq!(last_session(&dumped), LastSession::Dumped, "{dumped}");
        assert_eq!(
            last_session(dumped.trim_end()),
            LastSession::Dumped,
            "no trailing newline"
        );
        // A window thread still running while the crashed one waited for the monitor's
        // reply may log after the monitor's line.
        let logged_after = format!("{dumped}WARN  save_editor: a portrait did not load\n");
        assert_eq!(
            last_session(&logged_after),
            LastSession::Dumped,
            "{logged_after}"
        );
        // A second window's monitor can append into this window's log (the second start
        // renamed the first window's log away); this window's own clean close still wins.
        let closed_after = format!("{dumped}INFO  session: {CLOSING_LINE}\n");
        assert_eq!(
            last_session(&closed_after),
            LastSession::Closed,
            "{closed_after}"
        );
        let failed = format!(
            "{version}\n{}",
            log_line(
                Level::Error,
                "crash_dump",
                &"the crash dump could not be written: disk full"
            )
        );
        assert_eq!(last_session(&failed), LastSession::Unclean, "{failed}");
    }

    // ST-RUN-09
    #[test]
    fn open_log_reports_an_old_log_without_a_closing_line_as_unclean() {
        let dir = TestDir::new("unclean_old_log");
        fs::write(
            dir.path().join(LOG_FILE_NAME),
            "v\nERROR panic: thread 'main' panicked: boom\n  0: 0x7ff6a1230000+0x59c4\n",
        )
        .unwrap();
        assert_eq!(
            open_log(dir.path(), "v").unwrap().last_session,
            LastSession::Unclean
        );

        let dir = TestDir::new("clean_old_log");
        fs::write(
            dir.path().join(LOG_FILE_NAME),
            "v\nINFO  session: the window closed normally\n",
        )
        .unwrap();
        assert_eq!(
            open_log(dir.path(), "v").unwrap().last_session,
            LastSession::Closed
        );

        let dir = TestDir::new("no_old_log");
        assert_eq!(
            open_log(dir.path(), "v").unwrap().last_session,
            LastSession::Closed
        );
    }

    // ST-RUN-10: the log is append-mode, so a line the monitor appends from its own
    // process between two of the window's writes is kept rather than overwritten.
    #[test]
    fn open_log_appends_so_a_monitors_line_lands_between_the_windows() {
        let dir = TestDir::new("append");
        let mut opened = open_log(dir.path(), "v").unwrap();
        let mut monitor = OpenOptions::new()
            .append(true)
            .open(dir.path().join(LOG_FILE_NAME))
            .unwrap();
        monitor.write_all(b"monitor\n").unwrap();
        writeln!(opened.file, "window").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap(),
            "v\nmonitor\nwindow\n"
        );
    }

    // ST-RUN-09
    #[test]
    fn unclean_text_names_the_old_log() {
        let text = unclean_text(Path::new("C:\\data\\utw.old.log"), None);
        assert!(text.contains("did not close normally"), "{text}");
        assert!(text.contains("C:\\data\\utw.old.log"), "{text}");
    }

    // ST-RUN-10
    #[test]
    fn unclean_text_with_a_dump_names_the_log_and_the_dump() {
        let text = unclean_text(
            Path::new("C:\\data\\utw.old.log"),
            Some(Path::new("C:\\data\\utw.dmp")),
        );
        assert!(text.contains("did not close normally"), "{text}");
        assert!(text.contains("C:\\data\\utw.old.log"), "{text}");
        assert!(text.contains("C:\\data\\utw.dmp"), "{text}");
        assert_eq!(
            unclean_text(Path::new("C:\\data\\utw.old.log"), None),
            "The last session did not close normally. If it crashed, please post this file where you report problems: C:\\data\\utw.old.log. Your home folder is written in it as ~."
        );
    }

    // ST-RUN-09: the dialog names the file `open_log` renamed the previous log to.
    #[test]
    fn old_path_is_the_old_log_beside_the_log() {
        let installed = InstalledLog {
            path: Path::new("C:\\data").join(LOG_FILE_NAME),
            last_session: LastSession::Unclean,
        };
        assert_eq!(
            installed.old_path(),
            Path::new("C:\\data").join(OLD_LOG_FILE_NAME)
        );
    }

    // ST-RUN-10: the dialog names the dump only when the monitor said it wrote one and
    // the file is still there.
    #[test]
    fn dump_path_names_the_dump_beside_a_dumped_log_still_there() {
        let dir = TestDir::new("dump_path");
        fs::write(dir.path().join(DUMP_FILE_NAME), "dump").unwrap();
        let installed = InstalledLog {
            path: dir.path().join(LOG_FILE_NAME),
            last_session: LastSession::Dumped,
        };
        assert_eq!(installed.dump_path(), Some(dir.path().join(DUMP_FILE_NAME)));
        let installed = InstalledLog {
            last_session: LastSession::Unclean,
            ..installed
        };
        assert_eq!(installed.dump_path(), None);

        let dir = TestDir::new("dump_path_missing");
        let installed = InstalledLog {
            path: dir.path().join(LOG_FILE_NAME),
            last_session: LastSession::Dumped,
        };
        assert_eq!(installed.dump_path(), None);
    }

    // ST-RUN-09, ST-RUN-10: the window's start shows the notice only after a session
    // that did not close, naming the dump when the monitor wrote one.
    #[test]
    fn unclean_notice_is_shown_only_for_a_session_that_did_not_close() {
        let dir = TestDir::new("unclean_notice");
        fs::write(dir.path().join(DUMP_FILE_NAME), "dump").unwrap();
        let installed = InstalledLog {
            path: dir.path().join(LOG_FILE_NAME),
            last_session: LastSession::Closed,
        };
        assert_eq!(unclean_notice(&installed), None);

        let installed = InstalledLog {
            last_session: LastSession::Unclean,
            ..installed
        };
        assert_eq!(
            unclean_notice(&installed),
            Some(unclean_text(&dir.path().join(OLD_LOG_FILE_NAME), None))
        );

        let installed = InstalledLog {
            last_session: LastSession::Dumped,
            ..installed
        };
        assert_eq!(
            unclean_notice(&installed),
            Some(unclean_text(
                &dir.path().join(OLD_LOG_FILE_NAME),
                Some(&dir.path().join(DUMP_FILE_NAME))
            ))
        );
    }

    // A process sets one global logger ever, and `main`'s verbosity test installs the
    // CLI's env_logger, so whichever of the two tests ran second in this process would
    // lose. This one drives `install_gui_logger` in a fresh child of the test binary,
    // the env flag keeping the child from re-spawning itself. The `None` case installs
    // nothing, so it is checked first, before the install.
    #[test]
    fn install_gui_logger_writes_the_version_line_and_then_what_is_logged() {
        const CHILD: &str = "STUDIO_TEST_GUI_LOG_INSTALL";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "gui_log::tests::install_gui_logger_writes_the_version_line_and_then_what_is_logged",
                ])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert!(status.success());
            return;
        }
        let dir = TestDir::new("install_log");
        assert!(install_gui_logger(None).is_none());
        let installed = install_gui_logger(Some(dir.path())).unwrap();
        assert_eq!(installed.path, dir.path().join("utw.log"));
        assert_eq!(installed.last_session, LastSession::Closed);
        log::info!("install probe");
        let written = fs::read_to_string(&installed.path).unwrap();
        let mut lines = written.lines();
        assert_eq!(
            lines.next().unwrap(),
            format!("{} {}", studio_core::app_name!(), version_label())
        );
        assert!(
            lines.any(|line| line.contains("install probe")),
            "{written}"
        );
    }
}
