//! The crash dump, for the crashes the panic hook never sees: a stack overflow or an
//! access violation in native code leaves a log that simply stops. The window starts a copy of its own exe as
//! a monitor (`--crash-monitor`, told its slot's number), connects to it over a socket file named after the
//! process id in the temp folder, and the `crash-handler` hook asks the connected
//! monitor for a dump when a crash comes: the monitor writes the slot's dump (`utw.dmp`,
//! `utw.n.dmp` for the nth copy) beside the slot's log, appends a line naming it, and
//! exits with the window's process (ST-RUN-12). The
//! crash test's trigger (`--crash-test stack`) is `overflow_the_stack`, run once the
//! monitor is up and before any window, so the test needs no display (ST-RUN-10).

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::Context;
use crash_handler::{CrashEventResult, CrashHandler};
use directories::BaseDirs;
use log::{Level, Record};
use studio_core::EXE_NAME;

use crate::gui_log;

/// `utw.dmp`'s name in the data location, beside `utw.log`: slot 1's dump. Slot n's is
/// `utw.n.dmp` (`instance::slot_file_name`).
pub(crate) const DUMP_FILE_NAME: &str = "utw.dmp";
/// What the monitor's line after a dump ends with (ST-RUN-10): the next start reads a
/// line by this ending to name the dump in the unclean-session dialog, the slot's
/// number sitting inside the line where `dump_written_line` spells it.
pub(crate) const DUMP_WRITTEN_ENDING: &str = "beside this log";
/// The monitor's line when the dump could not be written: the next start reports the
/// session as simply unclean.
const DUMP_FAILED_LINE: &str = "the crash dump could not be written";

/// The hidden argument the monitor is started with, followed by the data location and
/// the socket's path.
const MONITOR_ARG: &str = "--crash-monitor";
/// The hidden argument running the crash test's trigger, followed by which crash.
const CRASH_TEST_ARG: &str = "--crash-test";
/// How long `connect` waits for the monitor to accept its socket before giving up.
const MONITOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// The monitor's log line after writing the slot's dump (ST-RUN-10): it names the
/// dump's file (`utw.dmp` for slot 1, `utw.n.dmp` for slot n), and
/// `DUMP_WRITTEN_ENDING` is how the next start reads it, whatever the slot.
pub(crate) fn dump_written_line(slot: u32) -> String {
    format!(
        "the crash dump was written to {} {DUMP_WRITTEN_ENDING}",
        crate::instance::slot_file_name(DUMP_FILE_NAME, slot)
    )
}

/// A launch that is not a member's command: the monitor process the window spawns, or
/// the crash test's trigger (ST-RUN-10).
pub(crate) enum HiddenLaunch {
    /// `--crash-monitor <data dir> <socket path> <slot>`: serve dump requests over
    /// `socket`, writing `slot`'s dump and the log line into `data_dir`.
    Monitor {
        data_dir: PathBuf,
        socket: PathBuf,
        slot: u32,
    },
    /// `--crash-test stack`: run the window's start, then overflow the stack.
    CrashTest,
}

/// Reads the hidden launches before the command line is parsed: neither is a command a
/// member types, so neither is in `--help`. `args` includes `args[0]`; anything but the
/// two exact forms is `None`, and clap then reports it as today.
pub(crate) fn hidden_launch(args: &[OsString]) -> Option<HiddenLaunch> {
    match args {
        [_, arg, data_dir, socket, slot] if arg == MONITOR_ARG => {
            match slot.to_str().and_then(|slot| slot.parse::<u32>().ok()) {
                Some(slot @ 1..) => Some(HiddenLaunch::Monitor {
                    data_dir: data_dir.as_os_str().into(),
                    socket: socket.as_os_str().into(),
                    slot,
                }),
                _ => None,
            }
        }
        [_, arg, which] if arg == CRASH_TEST_ARG && which == "stack" => {
            Some(HiddenLaunch::CrashTest)
        }
        _ => None,
    }
}

/// The monitor's `ServerHandler`: the slot's dump beside the slot's log, replacing the
/// previous one, and the log line a later start reads to tell a dumped crash from an
/// unclean one.
struct DumpWriter {
    data_dir: PathBuf,
    slot: u32,
    home: Option<String>,
}

impl minidumper::ServerHandler for DumpWriter {
    fn create_minidump_file(&self) -> Result<(File, PathBuf), io::Error> {
        let path = self
            .data_dir
            .join(crate::instance::slot_file_name(DUMP_FILE_NAME, self.slot));
        Ok((File::create(&path)?, path))
    }

    fn on_minidump_created(
        &self,
        result: Result<minidumper::MinidumpBinary, minidumper::Error>,
    ) -> minidumper::LoopAction {
        let text = match &result {
            Ok(_) => dump_written_line(self.slot),
            Err(error) => format!("{DUMP_FAILED_LINE}: {error}"),
        };
        // The crashed thread waits in its hook for the reply the server sends after this
        // returns, so the window writes no closing line after this one; its other threads
        // may still log, which is why the window's log is opened for appending too.
        append_line(
            &self.data_dir,
            &crate::instance::slot_file_name(gui_log::LOG_FILE_NAME, self.slot),
            self.home.as_deref(),
            Level::Error,
            &text,
        );
        minidumper::LoopAction::Exit
    }

    fn on_message(&self, _kind: u32, _buffer: Vec<u8>) {}

    /// The window closed or was killed; the monitor's job is done.
    fn on_client_disconnected(&self, _num_clients: usize) -> minidumper::LoopAction {
        minidumper::LoopAction::Exit
    }
}

/// Appends one `crash_dump` line to `data_dir`'s `log_name` (the slot's log), the home
/// folder written as `~`. Append without create: a monitor whose window never opened a
/// log adds none.
fn append_line(data_dir: &Path, log_name: &str, home: Option<&str>, level: Level, text: &str) {
    let line = gui_log::redact_home(&gui_log::log_line(level, "crash_dump", &text), home);
    let appended = OpenOptions::new()
        .append(true)
        .open(data_dir.join(log_name))
        .and_then(|mut log| log.write_all(line.as_bytes()));
    // The monitor has no logger and the log is where it reports: a failed append is a
    // failure nobody can act on, the project's `log::debug!` case.
    if let Err(error) = appended {
        log::debug!("the crash monitor could not append its line to the log: {error}");
    }
}

/// The monitor process's whole job: bind `socket`, then serve dump requests until a
/// dump is written (or fails) or the window's end closes the connection. A failure to
/// bind or serve is appended to the slot's log, since the monitor's stdio is null and
/// the window sees only its exit code, and exits with 3, the code of a run that could
/// not do its job. `slot` is the window's copy number: it names the dump and log.
pub(crate) fn run_monitor(data_dir: &Path, socket: &Path, slot: u32) -> ExitCode {
    let home = BaseDirs::new().map(|dirs| dirs.home_dir().display().to_string());
    let run = minidumper::Server::with_name(socket.into()).and_then(|mut server| {
        server.run(
            Box::new(DumpWriter {
                data_dir: data_dir.to_path_buf(),
                slot,
                home: home.clone(),
            }),
            &AtomicBool::new(false),
            None,
        )
    });
    match run {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            append_line(
                data_dir,
                &crate::instance::slot_file_name(gui_log::LOG_FILE_NAME, slot),
                home.as_deref(),
                Level::Warn,
                &format!("the crash monitor stopped: {error}"),
            );
            ExitCode::from(3)
        }
    }
}

/// Waits for the monitor to bind and accept on `socket`, retrying every 20 ms for at
/// most `MONITOR_CONNECT_TIMEOUT`: the listener comes up a moment after the spawn. A
/// monitor `try_wait` reports already exited will never accept, so the wait fails at
/// once, naming its status.
fn connect(monitor: &mut Child, socket: &Path) -> anyhow::Result<minidumper::Client> {
    let deadline = Instant::now() + MONITOR_CONNECT_TIMEOUT;
    loop {
        let error = match minidumper::Client::with_name(socket.into()) {
            Ok(client) => return Ok(client),
            Err(error) => error,
        };
        if let Some(status) = monitor
            .try_wait()
            .context("cannot poll the crash monitor")?
        {
            anyhow::bail!("the crash monitor exited with {status} before the socket connected");
        }
        if Instant::now() >= deadline {
            return Err(anyhow::Error::from(error).context(format!(
                "the crash monitor did not accept {} within {MONITOR_CONNECT_TIMEOUT:?}",
                socket.display()
            )));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Spawns `program` as the monitor and attaches the `crash-handler` hook that asks it
/// for a dump. The socket is named after this process's id in the temp folder: a fixed
/// name would leave one of two windows unwatched, and a data-location path can pass
/// the 107 bytes a Windows socket path holds.
fn start_monitor(program: &Path, data_dir: &Path, slot: u32) -> anyhow::Result<CrashHandler> {
    let socket = std::env::temp_dir().join(format!("{EXE_NAME}-crash-{}.sock", std::process::id()));
    let mut monitor = Command::new(program)
        .arg(MONITOR_ARG)
        .arg(data_dir)
        .arg(&socket)
        .arg(slot.to_string())
        // A monitor holding a test's pipe would keep `Output` waiting.
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("cannot start the crash monitor {}", program.display()))?;
    let client = match connect(&mut monitor, &socket) {
        Ok(client) => client,
        Err(error) => {
            if let Err(error) = monitor.kill().and_then(|()| monitor.wait()) {
                log::debug!(
                    "the crash monitor could not be stopped after a failed connect: {error}"
                );
            }
            return Err(error);
        }
    };
    // SAFETY: the closure runs in the crashed process, where on Linux it is inside a
    // signal handler and may neither allocate nor lock; it only calls
    // `Client::request_dump`, which writes the request to the already-connected socket
    // (on Windows from a stack buffer, on Linux from the crash context crash-handler
    // keeps in a static) and reads the server's ack into a stack buffer (minidumper
    // 0.11, `ipc/client.rs`): send and receive calls only, which a signal handler may
    // make. A failed request returns `Handled(false)`, leaving the crash to the system
    // as without the hook.
    let handler = CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |context: &crash_handler::CrashContext| {
            CrashEventResult::Handled(client.request_dump(context).is_ok())
        })
    })?;
    // Yama's restricted ptrace lets only the named process dump us; the default lets any.
    #[cfg(target_os = "linux")]
    handler.set_ptracer(Some(monitor.id()));
    // `monitor` is dropped neither killed nor waited: the monitor exits on its own when
    // the client's socket closes.
    Ok(handler)
}

/// Starts the monitor and attaches the hook, returning the handler that keeps both.
/// On any failure (the exe's own path unknown, a monitor that does not start, a hook
/// that does not attach) one `Warn` line is logged through `logger` and the window
/// runs on with the log alone: the dump is a diagnostic, never a reason the window
/// does not open (ST-RUN-10). `program` is `std::env::current_exe()`'s result, passed
/// in so a test can name a file that is not an exe; `slot` is the copy's number, which
/// the monitor needs to name its dump and log (ST-RUN-12).
pub(crate) fn watch_crashes(
    program: io::Result<PathBuf>,
    data_dir: &Path,
    slot: u32,
    logger: &dyn log::Log,
) -> Option<CrashHandler> {
    let started = program
        .map_err(anyhow::Error::from)
        .and_then(|program| start_monitor(&program, data_dir, slot));
    match started {
        Ok(handler) => Some(handler),
        Err(error) => {
            logger.log(
                &Record::builder()
                    .level(Level::Warn)
                    .target("crash_dump")
                    .args(format_args!(
                        "the crash monitor did not start, so this session writes no crash dump: {error:#}"
                    ))
                    .build(),
            );
            None
        }
    }
}

/// `--crash-test stack`'s crash (ST-RUN-10): a stack overflow, the kind no panic hook
/// sees. ST-RUN-11 looks for this function's name in the dump's stack.
#[inline(never)]
pub(crate) fn overflow_the_stack(depth: u64) -> u64 {
    let mut array = [0_u8; 4096];
    std::hint::black_box(&mut array);
    // rustc's `unconditional_recursion` warning fires without a condition it cannot
    // see through.
    if std::hint::black_box(depth) == u64::MAX {
        return 0;
    }
    // Work after the call, so no build turns the recursion into a loop.
    overflow_the_stack(depth + 1) + u64::from(array[0])
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;

    use log::Metadata;
    use minidumper::ServerHandler;

    use super::*;
    use crate::gui_log::{InstalledLog, LOG_FILE_NAME, LastSession, open_log, unclean_text};
    use crate::instance::slot_file_name;
    use crate::test_dir::TestDir;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn hidden_launch_reads_the_monitor_and_crash_test_forms_only() {
        let Some(HiddenLaunch::Monitor {
            data_dir,
            socket,
            slot,
        }) = hidden_launch(&args(&[
            "studio",
            "--crash-monitor",
            "D:\\data",
            "D:\\temp\\s.sock",
            "1",
        ]))
        else {
            panic!("the monitor form did not parse")
        };
        assert_eq!(data_dir, PathBuf::from("D:\\data"));
        assert_eq!(socket, PathBuf::from("D:\\temp\\s.sock"));
        assert_eq!(slot, 1);
        assert!(matches!(
            hidden_launch(&args(&["studio", "--crash-test", "stack"])),
            Some(HiddenLaunch::CrashTest)
        ));
        for bad in [
            args(&["studio", "--crash-test", "other"]),
            args(&["studio", "--crash-test"]),
            args(&["studio", "--crash-monitor", "D:\\data"]),
            args(&["studio", "--crash-monitor", "D:\\data", "D:\\temp\\s.sock"]),
            args(&[
                "studio",
                "--crash-monitor",
                "D:\\data",
                "D:\\temp\\s.sock",
                "x",
            ]),
            args(&[
                "studio",
                "--crash-monitor",
                "D:\\data",
                "D:\\temp\\s.sock",
                "0",
            ]),
            args(&["studio", "team-compiler", "check"]),
            args(&[
                "studio",
                "team-compiler",
                "D:\\data",
                "D:\\temp\\s.sock",
                "1",
            ]),
        ] {
            assert!(hidden_launch(&bad).is_none(), "{bad:?}");
        }
    }

    /// A `log::Log` collecting each record as `level|target|text`, for the warning a
    /// failed monitor start leaves (ST-RUN-10).
    #[derive(Default)]
    struct CollectLog {
        records: Mutex<Vec<String>>,
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

        fn flush(&self) {}
    }

    // ST-RUN-10
    #[test]
    fn a_monitor_that_cannot_start_logs_a_warning_and_watches_nothing() {
        let dir = TestDir::new("monitor_start");
        let not_an_exe = dir.path().join("not-an-exe.txt");
        fs::write(&not_an_exe, "text").unwrap();
        let logger = CollectLog::default();
        assert!(
            watch_crashes(Ok(not_an_exe.clone()), dir.path(), 1, &logger).is_none(),
            "a program that is not an exe still watches crashes"
        );
        let records = logger.records.lock().unwrap();
        assert_eq!(records.len(), 1, "{records:?}");
        assert!(records[0].starts_with("Warn|crash_dump|"), "{}", records[0]);
        assert!(
            records[0].contains(&not_an_exe.display().to_string()),
            "the warning does not name the program: {}",
            records[0]
        );
    }

    // ST-RUN-10: the next start's dialog text is built from what the monitor leaves:
    // its line in `utw.log` and `utw.dmp` beside it.
    #[test]
    fn a_written_dump_appends_the_line_the_next_start_names() {
        let dir = TestDir::new("dump_written");
        let log_path = dir.path().join(LOG_FILE_NAME);
        fs::write(&log_path, "v\n").unwrap();
        let writer = DumpWriter {
            data_dir: dir.path().to_path_buf(),
            slot: 1,
            home: None,
        };
        let (file, path) = writer.create_minidump_file().unwrap();
        assert_eq!(path, dir.path().join(DUMP_FILE_NAME));
        let action = writer.on_minidump_created(Ok(minidumper::MinidumpBinary {
            file,
            path,
            contents: None,
        }));
        assert!(
            matches!(action, minidumper::LoopAction::Exit),
            "the monitor keeps running after its dump"
        );
        // A normal close disconnects the window's client: the monitor's only way out then.
        assert!(
            matches!(
                writer.on_client_disconnected(0),
                minidumper::LoopAction::Exit
            ),
            "the monitor outlives the window it watched"
        );
        let written = fs::read_to_string(&log_path).unwrap();
        let last = written.lines().next_back().unwrap();
        assert!(last.ends_with(&dump_written_line(1)), "{last}");

        let opened = open_log(dir.path(), 1, "v2").unwrap();
        assert_eq!(opened.last_session, LastSession::Dumped);
        let installed = InstalledLog {
            path: log_path,
            last_session: opened.last_session,
            slot: 1,
        };
        assert_eq!(installed.dump_path(), Some(dir.path().join(DUMP_FILE_NAME)));
        let text = unclean_text(&installed.old_path(), installed.dump_path().as_deref());
        assert!(text.contains("utw.old.log"), "{text}");
        assert!(text.contains(DUMP_FILE_NAME), "{text}");
    }

    // A monitor that cannot bind its socket says why in the log, since the window sees
    // only its exit code.
    #[test]
    fn a_monitor_that_cannot_bind_appends_its_error_and_exits_with_3() {
        let dir = TestDir::new("monitor_bind");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let socket = dir.path().join("no-such-folder").join("s.sock");
        let code = run_monitor(dir.path(), &socket, 1);
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(3)));
        let written = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
        let last = written.lines().next_back().unwrap();
        assert!(
            last.starts_with("WARN  crash_dump: the crash monitor stopped: "),
            "{last}"
        );
    }

    // ST-RUN-10: a dump that fails appends the failure's line with its error, and a
    // folder holding no `utw.log` gets none.
    #[test]
    fn a_failed_dump_appends_the_failure_line_and_no_log_gets_none() {
        let dir = TestDir::new("dump_failed");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let writer = DumpWriter {
            data_dir: dir.path().to_path_buf(),
            slot: 1,
            home: None,
        };
        let action =
            writer.on_minidump_created(Err(minidumper::Error::Io(io::Error::other("disk full"))));
        assert!(
            matches!(action, minidumper::LoopAction::Exit),
            "the monitor keeps running after a failed dump"
        );
        let written = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
        let last = written.lines().next_back().unwrap();
        assert!(last.contains(DUMP_FAILED_LINE), "{last}");
        assert!(last.contains("disk full"), "{last}");

        let dir = TestDir::new("dump_no_log");
        let writer = DumpWriter {
            data_dir: dir.path().to_path_buf(),
            slot: 1,
            home: None,
        };
        let dump = dir.path().join(DUMP_FILE_NAME);
        let file = File::create(&dump).unwrap();
        writer.on_minidump_created(Ok(minidumper::MinidumpBinary {
            file,
            path: dump,
            contents: None,
        }));
        assert!(
            !dir.path().join(LOG_FILE_NAME).exists(),
            "the monitor created a log none had opened"
        );
    }

    // The monitor's arguments carry the window's slot, and its appended line
    // goes to that slot's log, not the first copy's `utw.log`.
    // ST-RUN-12
    #[test]
    fn the_monitor_reads_its_slot_and_appends_to_the_slots_log() {
        let Some(HiddenLaunch::Monitor { slot, .. }) = hidden_launch(&args(&[
            "studio",
            "--crash-monitor",
            "D:\\data",
            "D:\\temp\\s.sock",
            "2",
        ])) else {
            panic!("the monitor form did not parse")
        };
        assert_eq!(slot, 2);
        assert_eq!(
            dump_written_line(2),
            "the crash dump was written to utw.2.dmp beside this log"
        );

        let dir = TestDir::new("monitor_slot");
        fs::write(dir.path().join(LOG_FILE_NAME), "first copy\n").unwrap();
        let slot2_log = slot_file_name(LOG_FILE_NAME, 2);
        fs::write(dir.path().join(&slot2_log), "v\n").unwrap();
        append_line(dir.path(), &slot2_log, None, Level::Error, "probe");
        assert_eq!(
            fs::read_to_string(dir.path().join(&slot2_log)).unwrap(),
            "v\nERROR crash_dump: probe\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap(),
            "first copy\n"
        );
    }
}
