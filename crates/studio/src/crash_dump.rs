//! The crash dump, for the crashes the panic hook never sees: a stack overflow or an
//! access violation in native code leaves a log that simply stops. The window starts a copy of its own exe as
//! a monitor (`--crash-monitor`, told its slot's number), connects to it over a socket file named after the
//! process id in the temp folder, and the `crash-handler` hook asks the connected
//! monitor for a dump when a crash comes: the monitor writes the slot's dump (`utw.dmp`,
//! `utw.n.dmp` for the nth copy) beside the slot's log, appends a line naming it, and
//! exits with the window's process (ST-RUN-12). The
//! crash test's triggers (`--crash-test stack`, `session`, `monitor`) are
//! `overflow_the_stack`, `session_test` and `monitor_test`, run once the monitor is up
//! and before any window, so the tests need no
//! display (ST-RUN-10, ST-REP-11, ST-REP-10).
//!
//! The monitor's other job is the window's session folder (`<data location>/sessions/`):
//! the window's session messages reach it as frames through its standard input, a pipe
//! the window opens when it starts the monitor, and the monitor writes each into the
//! folder as it arrives and appends `[end]` — `closed`, `crashed` or `ended` — once the
//! pipe's end says the window is gone. A monitor that dies mid-session is replaced
//! once: the window's link starts another (`monitor_restart`), which writes on into
//! the same folder.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Context;
use crash_handler::{CrashEventResult, CrashHandler};
use directories::BaseDirs;
use log::{Level, Record};
use studio_core::EXE_NAME;
use studio_core::repro::{Recorder, SessionEnd, SessionFrame, SessionWriter, read_frame};

use crate::gui_log;
use crate::session_link::SessionLink;

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
/// Not 2 s: on a loaded machine (a game or builds running) the monitor, a second start
/// of the whole binary, took longer to come up, and the session then ran with no crash
/// dump. The window waits this long only when the monitor is that slow.
const MONITOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
    /// `--crash-test <which>`: run the window's start, then the test's
    /// trigger (`stack` overflows the stack, `session` records two steps
    /// and exits without the closing message, `monitor` ends its monitor
    /// mid-session and lets the link replace it).
    CrashTest(CrashTest),
}

/// Which trigger `--crash-test` runs once the window's start is done.
pub(crate) enum CrashTest {
    /// `--crash-test stack`: overflow the stack past the monitor's start
    /// (ST-RUN-10).
    Stack,
    /// `--crash-test session`: record two steps through the session link,
    /// then end the process with no closing message (ST-REP-11).
    Session,
    /// `--crash-test monitor`: record a step, end the monitor, record a
    /// second step into the same folder through the new monitor, then
    /// close the session (ST-REP-10).
    Monitor,
}

/// Reads the hidden launches before the command line is parsed: neither is a command a
/// member types, so neither is in `--help`. `args` includes `args[0]`; anything but the
/// exact forms is `None`, and clap then reports it as today.
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
            Some(HiddenLaunch::CrashTest(CrashTest::Stack))
        }
        [_, arg, which] if arg == CRASH_TEST_ARG && which == "session" => {
            Some(HiddenLaunch::CrashTest(CrashTest::Session))
        }
        [_, arg, which] if arg == CRASH_TEST_ARG && which == "monitor" => {
            Some(HiddenLaunch::CrashTest(CrashTest::Monitor))
        }
        _ => None,
    }
}

/// The monitor's `ServerHandler`: the slot's dump beside the slot's log, replacing the
/// previous one, and the log line a later start reads to tell a dumped crash from an
/// unclean one. `dump_requested` marks the session's `[end]` `crashed`, whatever the
/// write's outcome: the window's hook asks only from a crash.
struct DumpWriter {
    data_dir: PathBuf,
    slot: u32,
    home: Option<String>,
    dump_requested: Arc<AtomicBool>,
}

impl minidumper::ServerHandler for DumpWriter {
    fn create_minidump_file(&self) -> Result<(File, PathBuf), io::Error> {
        self.dump_requested.store(true, Ordering::Relaxed);
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

    /// The socket's spare channel is not the session's link: the server reads a message
    /// in one receive, so a kept save (5 to 11 MB) could arrive cut at the socket's
    /// buffer. The session's frames travel the monitor's standard input instead, read
    /// whole with `read_exact`, and the pipe's end tells the monitor the window went
    /// away, which no socket message can.
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
    let line = studio_core::redact_home(&gui_log::log_line(level, "crash_dump", &text), home);
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
/// dump is written (or fails) or the window's end closes the connection; and write the
/// window's session folder — a thread of its own reads the session's frames from
/// `input` (the monitor's standard input) until the window's end of the pipe closes,
/// and the journal's `[end]` is set then. A failure to bind or serve is appended to
/// the slot's log, since the monitor's stdio is null and the window sees only its
/// exit code, and exits with 3, the code of a run that could not do its job. The bind
/// failure's line goes at `Info`: the window retries the start, and the one warning a
/// failed monitor earns belongs to `watch_crashes`, whose retry also failed. `slot`
/// is the window's copy number: it names the dump and log.
pub(crate) fn run_monitor(
    data_dir: &Path,
    socket: &Path,
    slot: u32,
    input: &mut (impl Read + Send),
) -> ExitCode {
    let home = BaseDirs::new().map(|dirs| dirs.home_dir().display().to_string());
    // The bind comes first: a monitor that cannot bind returns at once and the
    // window's start is tried again, rather than waiting the reader's pipe out.
    let mut server = match minidumper::Server::with_name(socket.into()) {
        Ok(server) => server,
        Err(error) => {
            append_line(
                data_dir,
                &crate::instance::slot_file_name(gui_log::LOG_FILE_NAME, slot),
                home.as_deref(),
                Level::Info,
                &format!("the crash monitor stopped: {error}"),
            );
            return ExitCode::from(3);
        }
    };
    let dump_requested = Arc::new(AtomicBool::new(false));
    let (run, session) = std::thread::scope(|scope| {
        let reader = scope.spawn(|| read_session(input, data_dir, slot, home.as_deref()));
        let run = server.run(
            Box::new(DumpWriter {
                data_dir: data_dir.to_path_buf(),
                slot,
                home: home.clone(),
                dump_requested: dump_requested.clone(),
            }),
            &AtomicBool::new(false),
            None,
        );
        // The pipe's end, not the socket's, is the window gone away: the reader is
        // joined until it ends, so a monitor whose socket closed first writes the
        // session on to the last message.
        let session = reader
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        (run, session)
    });
    end_session(
        session,
        dump_requested.load(Ordering::Relaxed),
        data_dir,
        slot,
        home.as_deref(),
    );
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

/// What the session reader kept from the pipe: the writer the window's header made
/// (`None` when none arrived) and the ending the window's closing message told
/// (`Closed` or `Crashed`), `None` when no closing message arrived.
struct SessionRead {
    writer: Option<SessionWriter>,
    told: Option<SessionEnd>,
}

/// The window's session, read frame by frame until the pipe ends: the header makes the
/// writer under `<data location>/sessions`, each message is written and the closing
/// message's ending is remembered. A frame before the header, a second header or a failed write is one
/// Warn line in the slot's log and the reader goes on with the next frame; a frame
/// that does not decode is one Warn line and ends the reading as the pipe's end does.
fn read_session(
    input: &mut impl Read,
    data_dir: &Path,
    slot: u32,
    home: Option<&str>,
) -> SessionRead {
    let log = crate::instance::slot_file_name(gui_log::LOG_FILE_NAME, slot);
    let mut session = SessionRead {
        writer: None,
        told: None,
    };
    loop {
        let frame = match read_frame(input) {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(error) => {
                append_line(
                    data_dir,
                    &log,
                    home,
                    Level::Warn,
                    &format!("a session frame could not be read: {error}"),
                );
                break;
            }
        };
        match frame {
            SessionFrame::Header(header) => {
                if session.writer.is_some() {
                    append_line(
                        data_dir,
                        &log,
                        home,
                        Level::Warn,
                        "a second session header arrived; the session keeps the first",
                    );
                } else {
                    session.writer = Some(SessionWriter::new(&data_dir.join("sessions"), header));
                }
            }
            SessionFrame::Message(message) => match &mut session.writer {
                Some(writer) => {
                    if let Err(error) = writer.write(&message) {
                        append_line(
                            data_dir,
                            &log,
                            home,
                            Level::Warn,
                            &format!("a session message could not be written: {error:#}"),
                        );
                    }
                }
                None => append_line(
                    data_dir,
                    &log,
                    home,
                    Level::Warn,
                    "a session message arrived before the session's header and was not written",
                ),
            },
            SessionFrame::Closed | SessionFrame::Crashed => {
                if session.writer.is_none() {
                    append_line(
                        data_dir,
                        &log,
                        home,
                        Level::Warn,
                        "the session's closing message arrived before its header",
                    );
                }
                session.told = Some(if matches!(frame, SessionFrame::Crashed) {
                    SessionEnd::Crashed
                } else {
                    SessionEnd::Closed
                });
            }
        }
    }
    session
}

/// The session's `[end]` from the monitor's two facts: `crashed` when a dump was
/// requested (the window's hook asks only from a crash), else the `told` the
/// window's closing message gave (`closed` or `crashed` — a caught panic is a
/// crash to the member), else `ended`, the window gone with neither.
fn session_end(dump_requested: bool, told: Option<SessionEnd>) -> SessionEnd {
    if dump_requested {
        SessionEnd::Crashed
    } else {
        told.unwrap_or(SessionEnd::Ended)
    }
}

/// The session's `[end]` written once the window is gone. No header means no writer
/// was made, so a session that recorded nothing leaves no folder at all.
fn end_session(
    session: SessionRead,
    dump_requested: bool,
    data_dir: &Path,
    slot: u32,
    home: Option<&str>,
) {
    let Some(mut writer) = session.writer else {
        return;
    };
    if let Err(error) = writer.end(session_end(dump_requested, session.told)) {
        append_line(
            data_dir,
            &crate::instance::slot_file_name(gui_log::LOG_FILE_NAME, slot),
            home,
            Level::Warn,
            &format!("the session's end could not be written: {error:#}"),
        );
    }
}

/// How a monitor start can fail, for the one distinction `watch_crashes` needs:
/// whether a second try is worth it (ST-RUN-13).
enum StartFailure {
    /// The spawn refused, or the monitor exited before its socket connected: a
    /// cause that passes (an antivirus scanning the exe as it starts) still leaves
    /// the window watched, so the start is tried once more.
    AtOnce(anyhow::Error),
    /// The monitor stayed up but did not accept within `MONITOR_CONNECT_TIMEOUT`:
    /// not tried again, since the window waits that long before opening and a
    /// second wait would double it.
    NoAnswer(anyhow::Error),
    /// Anything else: the exe's own path was unknown, the monitor could not be
    /// polled, or the hook did not attach.
    Other(anyhow::Error),
}

impl StartFailure {
    /// The error under the kind, for the warning that names it.
    fn error(&self) -> &anyhow::Error {
        match self {
            Self::AtOnce(error) | Self::NoAnswer(error) | Self::Other(error) => error,
        }
    }
}

/// Waits for the monitor to bind and accept on `socket`, retrying every 20 ms for at
/// most `MONITOR_CONNECT_TIMEOUT`: the listener comes up a moment after the spawn. A
/// monitor `try_wait` reports already exited will never accept, so the wait fails at
/// once (`StartFailure::AtOnce`), naming its status; the deadline's is `NoAnswer`.
fn connect(monitor: &mut Child, socket: &Path) -> Result<minidumper::Client, StartFailure> {
    let deadline = Instant::now() + MONITOR_CONNECT_TIMEOUT;
    loop {
        let error = match minidumper::Client::with_name(socket.into()) {
            Ok(client) => return Ok(client),
            Err(error) => error,
        };
        if let Some(status) = monitor
            .try_wait()
            .context("cannot poll the crash monitor")
            .map_err(StartFailure::Other)?
        {
            return Err(StartFailure::AtOnce(anyhow::anyhow!(
                "the crash monitor exited with {status} before the socket connected"
            )));
        }
        if Instant::now() >= deadline {
            return Err(StartFailure::NoAnswer(anyhow::Error::from(error).context(
                format!(
                    "the crash monitor did not accept {} within {MONITOR_CONNECT_TIMEOUT:?}",
                    socket.display()
                ),
            )));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The monitor a window is watched by, as `start_monitor` hands it up: the
/// `CrashHandler` that asks it for a dump, its standard input the session's
/// frames travel, and the `Child` itself, kept so a test can end it.
pub(crate) struct Watch {
    /// The attached crash hook; dropped, it detaches.
    pub(crate) handler: CrashHandler,
    /// The monitor's standard input: the session's pipe, whose end tells
    /// the monitor the window went away.
    pub(crate) pipe: ChildStdin,
    /// The monitor process. Dropping it leaves it running — it exits when
    /// the socket the handler held closes; a test kills it.
    pub(crate) child: Child,
}

/// Spawns `program` as the monitor and attaches the `crash-handler` hook that asks it
/// for a dump, or fails with the `StartFailure` kind `watch_crashes` reads for its
/// second try. The socket is named after this process's id in the temp folder: a fixed
/// name would leave one of two windows unwatched, and a data-location path can pass
/// the 107 bytes a Windows socket path holds. A stale file under that name needs no
/// removal here: `minidumper`'s `Server::with_name` deletes its path before binding.
fn start_monitor(program: &Path, data_dir: &Path, slot: u32) -> Result<Watch, StartFailure> {
    let socket = std::env::temp_dir().join(format!("{EXE_NAME}-crash-{}.sock", std::process::id()));
    let mut monitor = Command::new(program)
        .arg(MONITOR_ARG)
        .arg(data_dir)
        .arg(&socket)
        .arg(slot.to_string())
        // A monitor holding a test's pipe would keep `Output` waiting.
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("cannot start the crash monitor {}", program.display()))
        .map_err(StartFailure::AtOnce)?;
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
    })
    .map_err(|error| StartFailure::Other(error.into()))?;
    // Yama's restricted ptrace lets only the named process dump us; the default lets any.
    #[cfg(target_os = "linux")]
    handler.set_ptracer(Some(monitor.id()));
    // `monitor` is neither killed nor waited: it exits on its own when the
    // client's socket closes.
    let pipe = monitor
        .stdin
        .take()
        .expect("the monitor's standard input was piped");
    Ok(Watch {
        handler,
        pipe,
        child: monitor,
    })
}

/// Runs `start` and, only when it fails at once, runs it once more and returns the
/// second result (ST-RUN-13): a cause that passes, an antivirus scanning the exe as
/// it starts, still leaves the session watched. The first failure is logged through
/// `logger` at `Info` so the log shows it happened; any other failure returns at
/// once, since `NoAnswer` already waited out `MONITOR_CONNECT_TIMEOUT` and the rest
/// cannot pass on a retry.
fn start_monitor_once_more<T>(
    start: impl Fn() -> Result<T, StartFailure>,
    logger: &dyn log::Log,
) -> Result<T, StartFailure> {
    match start() {
        Err(StartFailure::AtOnce(error)) => {
            logger.log(
                &Record::builder()
                    .level(Level::Info)
                    .target("crash_dump")
                    .args(format_args!(
                        "the crash monitor did not start, so it is tried once more: {error:#}"
                    ))
                    .build(),
            );
            start()
        }
        done => done,
    }
}

/// Starts the monitor and attaches the hook, returning the `Watch` that
/// keeps both: the handler until it is dropped, the monitor's standard
/// input — the pipe the window's session frames will travel — and the
/// `Child`. A start that fails at once is tried once more
/// (`start_monitor_once_more`, ST-RUN-13). On failure (the exe's own path unknown, a
/// monitor that does not start twice or does not answer, a hook that does not attach)
/// one `Warn` line is logged through `logger` and the window
/// runs on with the log alone: the dump is a diagnostic, never a reason the window
/// does not open (ST-RUN-10). `program` is `std::env::current_exe()`'s result, passed
/// in so a test can name a file that is not an exe; `slot` is the copy's number, which
/// the monitor needs to name its dump and log (ST-RUN-12).
pub(crate) fn watch_crashes(
    program: io::Result<PathBuf>,
    data_dir: &Path,
    slot: u32,
    logger: &dyn log::Log,
) -> Option<Watch> {
    let started = program
        .map_err(|error| StartFailure::Other(error.into()))
        .and_then(|program| {
            start_monitor_once_more(|| start_monitor(&program, data_dir, slot), logger)
        });
    match started {
        Ok(handler) => Some(handler),
        Err(failure) => {
            logger.log(
                &Record::builder()
                    .level(Level::Warn)
                    .target("crash_dump")
                    .args(format_args!(
                        "the crash monitor did not start, so this session writes no crash dump: {:#}",
                        failure.error()
                    ))
                    .build(),
            );
            None
        }
    }
}

/// The session link's `restart` after its pipe broke: `handler`, the dead
/// monitor's hook, is dropped first — `crash-handler` holds only one attach
/// at a time — then `watch_crashes` runs a new monitor. Its pipe is the
/// link's new target; its handler is kept for the thread's end, which
/// detaches it as `main` did after `close`. A monitor that starts logs one
/// Info line, so a restart that made it and the fallback are told apart in
/// the log. `None` is a monitor that did not start, which hands the folder
/// to the window's own writer.
pub(crate) fn monitor_restart(
    data_dir: &Path,
    slot: u32,
    handler: CrashHandler,
) -> Box<dyn FnMut() -> Option<Box<dyn Write + Send>> + Send> {
    let data_dir = data_dir.to_path_buf();
    let mut handler = Some(handler);
    Box::new(move || {
        drop(handler.take());
        watch_crashes(std::env::current_exe(), &data_dir, slot, log::logger()).map(|watch| {
            handler = Some(watch.handler);
            // `watch.child` drops here, left running like the first monitor:
            // it exits when the socket the handler held closes.
            log::info!("a new crash monitor was started after the session's pipe broke");
            Box::new(watch.pipe) as Box<dyn Write + Send>
        })
    })
}

/// The session link's `restart` for a window whose monitor never started:
/// none is asked for — the first try already had its second
/// (`start_monitor_once_more`) — so the call gives no pipe back and the
/// window's own writer runs.
pub(crate) fn no_restart() -> Box<dyn FnMut() -> Option<Box<dyn Write + Send>> + Send> {
    Box::new(|| None)
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

/// The bound of every wait on another process, the limit `wait_until` is given.
const WAIT_LIMIT: Duration = Duration::from_secs(120);

/// `done` retried every 20 ms until it holds or `limit` passes, `false` past it.
fn wait_until(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        if done() {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// `folder`'s journal holds `steps` `[[step]]` blocks.
fn journal_holds_steps(folder: &Path, steps: usize) -> bool {
    fs::read_to_string(folder.join("journal.toml"))
        .map(|journal| journal.matches("[[step]]").count() >= steps)
        .unwrap_or(false)
}

/// The session folder under `sessions` whose journal holds `steps`
/// `[[step]]` blocks, `None` when none does inside the wait.
fn await_steps(sessions: &Path, steps: usize) -> Option<PathBuf> {
    let mut found = None;
    wait_until(WAIT_LIMIT, || {
        found = fs::read_dir(sessions).ok().and_then(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .find(|folder| journal_holds_steps(folder, steps))
        });
        found.is_some()
    });
    found
}

/// `--crash-test session`'s trigger (ST-REP-11): two steps through the
/// recorder — the link's thread sends them to the monitor, which writes them
/// into the session folder — then the process ends with no closing message.
/// `std::process::exit` runs no destructors, so the queue's sender never
/// drops, the sender thread dies mid-wait and the pipe's own end is the
/// monitor's only sign the window is gone: `[end]` reads `ended`. The exit
/// waits on the steps' arrival, counted in the folder, so it cannot cut a
/// message's send short; it is bounded like every wait on another process.
pub(crate) fn session_test(recorder: &Recorder, data_dir: Option<&Path>) -> ! {
    recorder.step("stub", "the session test's first step");
    recorder.step("stub", "the session test's second step");
    let Some(sessions) = data_dir.map(|dir| dir.join("sessions")) else {
        log::error!("the session test ran without a data location");
        std::process::exit(3);
    };
    if await_steps(&sessions, 2).is_none() {
        log::error!("the session test's steps never reached the session folder");
        std::process::exit(3);
    }
    std::process::exit(0);
}

/// `--crash-test monitor`'s trigger (ST-REP-10): a first step through the
/// recorder, waited on in the session folder; then `monitor` ended —
/// `wait` reaps the kill, so the pipe's break, not the kill's timing, is
/// what the link's next send meets — a second step waited on in the same
/// folder, which the new monitor's writer adopts, and the session closed
/// `closed` as `main` closes it. `link` or `monitor` `None` — a run with
/// no data location or one whose monitor never started — is a test that
/// cannot run, exit 3 as `session_test` does.
pub(crate) fn monitor_test(
    recorder: &Recorder,
    data_dir: Option<&Path>,
    link: Option<SessionLink>,
    monitor: Option<Child>,
) -> ! {
    let (Some(sessions), Some(link), Some(mut monitor)) =
        (data_dir.map(|dir| dir.join("sessions")), link, monitor)
    else {
        log::error!("the monitor test ran without a data location, a session link or a monitor");
        std::process::exit(3);
    };
    recorder.step("stub", "the monitor test's first step");
    let Some(folder) = await_steps(&sessions, 1) else {
        log::error!("the monitor test's first step never reached the session folder");
        std::process::exit(3);
    };
    if let Err(error) = monitor.kill().and_then(|()| monitor.wait()) {
        log::error!("the monitor test could not end its monitor: {error}");
        std::process::exit(3);
    }
    recorder.step("stub", "the monitor test's second step");
    if !wait_until(WAIT_LIMIT, || journal_holds_steps(&folder, 2)) {
        log::error!("the monitor test's second step never reached the session folder");
        std::process::exit(3);
    }
    link.close(SessionEnd::Closed);
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::sync::Mutex;

    use log::Metadata;
    use minidumper::ServerHandler;

    use studio_core::repro::{
        SessionFile, SessionFrame, SessionHeader, SessionMessage, write_frame,
    };

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
            Some(HiddenLaunch::CrashTest(CrashTest::Stack))
        ));
        assert!(matches!(
            hidden_launch(&args(&["studio", "--crash-test", "session"])),
            Some(HiddenLaunch::CrashTest(CrashTest::Session))
        ));
        assert!(matches!(
            hidden_launch(&args(&["studio", "--crash-test", "monitor"])),
            Some(HiddenLaunch::CrashTest(CrashTest::Monitor))
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
            args(&["studio", "compiler", "check"]),
            args(&["studio", "compiler", "D:\\data", "D:\\temp\\s.sock", "1"]),
        ] {
            assert!(hidden_launch(&bad).is_none(), "{bad:?}");
        }
    }

    // The wait is over once its limit is past: `false` then, `done` not asked again.
    #[test]
    fn wait_until_ends_false_once_the_limit_passes() {
        let calls = Cell::new(0);
        assert!(
            !wait_until(Duration::ZERO, || {
                calls.set(calls.get() + 1);
                calls.get() > 1
            }),
            "the limit was past after the first answer"
        );
    }

    // `journal_holds_steps` counts the journal's `[[step]]` blocks, a folder with no
    // journal holding none.
    #[test]
    fn journal_holds_steps_counts_the_step_blocks() {
        let dir = TestDir::new("journal_steps");
        let folder = dir.path().join("session");
        fs::create_dir_all(&folder).unwrap();
        assert!(
            !journal_holds_steps(&folder, 1),
            "no journal.toml: no steps held"
        );
        fs::write(folder.join("journal.toml"), "[[step]]\nwhat = \"a step\"\n").unwrap();
        assert!(journal_holds_steps(&folder, 1));
        assert!(!journal_holds_steps(&folder, 2), "one block, not two");
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

    // ST-RUN-10, ST-RUN-13
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
        assert_eq!(records.len(), 2, "{records:?}");
        assert!(records[0].starts_with("Info|crash_dump|"), "{}", records[0]);
        assert!(
            records[0].contains(&not_an_exe.display().to_string()),
            "the first try's note does not name the program: {}",
            records[0]
        );
        assert!(records[1].starts_with("Warn|crash_dump|"), "{}", records[1]);
        assert!(
            records[1].contains(&not_an_exe.display().to_string()),
            "the warning does not name the program: {}",
            records[1]
        );
    }

    // ST-RUN-13's "exits at once both times", with a real child: the test
    // binary itself refuses the monitor's arguments (libtest reports an
    // unrecognized option) and exits at once, so `connect` reads the exit as
    // `AtOnce`, the start is tried once more, and the log holds the retry's
    // Info then the one Warn. Nothing watches.
    #[test]
    fn a_real_child_that_exits_at_once_is_retried_and_gives_up() {
        let dir = TestDir::new("monitor_real_exit");
        let logger = CollectLog::default();
        assert!(
            watch_crashes(std::env::current_exe(), dir.path(), 1, &logger).is_none(),
            "a monitor that exits at once still watches crashes"
        );
        let records = logger.records.lock().unwrap();
        assert_eq!(records.len(), 2, "{records:?}");
        assert!(records[0].starts_with("Info|crash_dump|"), "{records:?}");
        assert!(
            records[0].contains("exited with"),
            "the first try's note is the child's own exit: {}",
            records[0]
        );
        assert!(records[1].starts_with("Warn|crash_dump|"), "{records:?}");
        assert!(records[1].contains("exited with"), "{records:?}");
    }

    // ST-RUN-13
    #[test]
    fn a_start_failed_at_once_is_tried_once_more() {
        let calls = Cell::new(0);
        let logger = CollectLog::default();
        let result: Result<(), StartFailure> = start_monitor_once_more(
            || {
                calls.set(calls.get() + 1);
                match calls.get() {
                    1 => Err(StartFailure::AtOnce(anyhow::anyhow!("gone"))),
                    _ => Ok(()),
                }
            },
            &logger,
        );
        assert!(result.is_ok(), "the second start's success is not returned");
        assert_eq!(calls.get(), 2, "the start did not run twice");
        // ST-RUN-13's "the log holds no warning": the first try's note only.
        let records = logger.records.lock().unwrap();
        assert_eq!(records.len(), 1, "{records:?}");
        assert!(records[0].starts_with("Info|crash_dump|"), "{}", records[0]);
    }

    // ST-RUN-13
    #[test]
    fn a_start_failed_at_once_twice_returns_the_second_error() {
        let calls = Cell::new(0);
        let result: Result<(), StartFailure> = start_monitor_once_more(
            || {
                calls.set(calls.get() + 1);
                Err(StartFailure::AtOnce(anyhow::anyhow!(
                    "gone {}",
                    calls.get()
                )))
            },
            &CollectLog::default(),
        );
        assert_eq!(calls.get(), 2, "the start did not run twice");
        let Err(StartFailure::AtOnce(error)) = result else {
            panic!("a twice-failed start is not returned as AtOnce")
        };
        assert_eq!(format!("{error:#}"), "gone 2");
    }

    // Anything but a failure at once (no answer within the timeout, or
    // the rest) comes back after the one start, error and kind unchanged.
    // ST-RUN-13
    #[test]
    fn a_start_without_an_answer_is_not_tried_again() {
        type Failure = fn() -> StartFailure;
        let cases: [(Failure, &str); 2] = [
            (
                || StartFailure::NoAnswer(anyhow::anyhow!("silent")),
                "silent",
            ),
            (|| StartFailure::Other(anyhow::anyhow!("odd")), "odd"),
        ];
        for (make, text) in cases {
            let calls = Cell::new(0);
            let logger = CollectLog::default();
            let result: Result<(), StartFailure> = start_monitor_once_more(
                || {
                    calls.set(calls.get() + 1);
                    Err(make())
                },
                &logger,
            );
            assert_eq!(calls.get(), 1, "a non-at-once failure was retried");
            let Err(failure) = result else {
                panic!("a non-at-once failure came back as a success")
            };
            assert_eq!(format!("{:#}", failure.error()), text);
            assert!(
                logger.records.lock().unwrap().is_empty(),
                "an unretried failure left an Info record"
            );
        }
    }

    // The next start's dialog text is built from what the monitor leaves:
    // its line in `utw.log` and `utw.dmp` beside it.
    // ST-RUN-10
    #[test]
    fn a_written_dump_appends_the_line_the_next_start_names() {
        let dir = TestDir::new("dump_written");
        let log_path = dir.path().join(LOG_FILE_NAME);
        fs::write(&log_path, "v\n").unwrap();
        let writer = DumpWriter {
            data_dir: dir.path().to_path_buf(),
            slot: 1,
            home: None,
            dump_requested: Arc::new(AtomicBool::new(false)),
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

    /// An input whose read panics: the session reader must never start on it.
    struct PanicRead;

    impl Read for PanicRead {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            panic!("the session reader started before the bind")
        }
    }

    // A monitor that cannot bind its socket says why in the log, since the window sees
    // only its exit code, at Info: the window retries the start, and the one warning
    // belongs to a start that failed twice. It returns before the session reader
    // starts: `PanicRead` would panic otherwise.
    #[test]
    fn a_monitor_that_cannot_bind_appends_its_error_and_exits_with_3() {
        let dir = TestDir::new("monitor_bind");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let socket = dir.path().join("no-such-folder").join("s.sock");
        let code = run_monitor(dir.path(), &socket, 1, &mut PanicRead);
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(3)));
        let written = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
        let last = written.lines().next_back().unwrap();
        assert!(
            last.starts_with("INFO  crash_dump: the crash monitor stopped: "),
            "{last}"
        );
    }

    // A dump that fails appends the failure's line with its error, and a
    // folder holding no `utw.log` gets none.
    // ST-RUN-10
    #[test]
    fn a_failed_dump_appends_the_failure_line_and_no_log_gets_none() {
        let dir = TestDir::new("dump_failed");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let writer = DumpWriter {
            data_dir: dir.path().to_path_buf(),
            slot: 1,
            home: None,
            dump_requested: Arc::new(AtomicBool::new(false)),
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
            dump_requested: Arc::new(AtomicBool::new(false)),
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

    /// A session header: `started` fixed at the epoch, built through `SystemTime`
    /// (jiff is not a `studio` dependency).
    fn header(pid: u32) -> SessionHeader {
        SessionHeader {
            version: "v-test".to_owned(),
            os: "test-os".to_owned(),
            started: std::time::SystemTime::UNIX_EPOCH.try_into().unwrap(),
            pid,
        }
    }

    /// A stub session's frames: the header, a step, a kept input and a kept output
    /// written twice to one path.
    fn frames(pid: u32) -> Vec<SessionFrame> {
        vec![
            SessionFrame::Header(header(pid)),
            SessionFrame::Message(SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "opened C:/Data/in/EDIT00000000 (10 bytes)".to_owned(),
                op: None,
            }),
            SessionFrame::Message(SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(b"INPUT_BYTES".as_slice().into()),
            }),
            SessionFrame::Message(SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(b"WRITE_A".as_slice().into()),
            }),
            SessionFrame::Message(SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(b"WRITE_B_BYTES".as_slice().into()),
            }),
        ]
    }

    /// `frames` as the pipe's bytes, `end` adding the closing message it
    /// names (`Some(Ended)` writes none, as the pipe's end does).
    fn session_bytes(frames: &[SessionFrame], end: Option<SessionEnd>) -> Vec<u8> {
        let mut bytes = Vec::new();
        for frame in frames {
            write_frame(&mut bytes, frame).unwrap();
        }
        match end {
            Some(SessionEnd::Closed) => write_frame(&mut bytes, &SessionFrame::Closed).unwrap(),
            Some(SessionEnd::Crashed) => write_frame(&mut bytes, &SessionFrame::Crashed).unwrap(),
            Some(SessionEnd::Ended) | None => {}
        }
        bytes
    }

    /// The one session folder under `dir`'s `sessions`.
    fn session_folder(dir: &TestDir) -> PathBuf {
        let sessions = dir.path().join("sessions");
        let mut folders = fs::read_dir(&sessions).unwrap();
        let folder = folders.next().unwrap().unwrap().path();
        assert!(
            folders.next().is_none(),
            "{sessions:?} holds more than one session folder"
        );
        folder
    }

    // The pipe's frames land in the session folder — the step, the kept input, the
    // output's second write — and `[end]` says how it ended: the `told` the window's
    // closing message gave (`closed`, or `crashed` after a panic it caught), `ended`
    // without one, `crashed` on a dump request whatever it told.
    // ST-REP-06, ST-REP-07, ST-REP-10
    #[test]
    fn the_session_is_written_from_the_pipe_and_ended() {
        for (told, dumped, how) in [
            (Some(SessionEnd::Closed), false, "closed"),
            (Some(SessionEnd::Crashed), false, "crashed"),
            (None, false, "ended"),
            (Some(SessionEnd::Closed), true, "crashed"),
            (Some(SessionEnd::Crashed), true, "crashed"),
        ] {
            let dir = TestDir::new(&format!("session_pipe_{told:?}_{dumped}"));
            let bytes = session_bytes(&frames(4242), told);
            let session = read_session(&mut &bytes[..], dir.path(), 1, None);
            let dump_requested = Arc::new(AtomicBool::new(false));
            if dumped {
                let writer = DumpWriter {
                    data_dir: dir.path().to_path_buf(),
                    slot: 1,
                    home: None,
                    dump_requested: dump_requested.clone(),
                };
                writer.create_minidump_file().unwrap();
            }
            end_session(
                session,
                dump_requested.load(Ordering::Relaxed),
                dir.path(),
                1,
                None,
            );

            let folder = session_folder(&dir);
            let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
            assert_eq!(journal.matches("[[step]]").count(), 1, "{journal}");
            assert_eq!(journal.matches("[[input]]").count(), 1, "{journal}");
            assert_eq!(journal.matches("[[output]]").count(), 2, "{journal}");
            assert!(
                journal.contains(&format!("how = \"{how}\"")),
                "[end] not written as {how}: {journal}"
            );
            assert_eq!(
                fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
                b"INPUT_BYTES"
            );
            assert_eq!(
                fs::read(folder.join("outputs/0-EDIT00000000")).unwrap(),
                b"WRITE_B_BYTES",
                "the output's second write is the one kept"
            );
        }
    }

    // A step before the header is one Warn line in the slot's log and reaches no
    // folder; the session after the header is written whole, ended `closed`.
    #[test]
    fn a_message_before_the_header_warns_and_is_dropped() {
        let dir = TestDir::new("session_early_step");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &SessionFrame::Message(SessionMessage::Step {
                seconds: 0.0,
                tool: "stub".to_owned(),
                what: "a step before the header".to_owned(),
                op: None,
            }),
        )
        .unwrap();
        bytes.extend(session_bytes(&frames(4242), Some(SessionEnd::Closed)));

        let session = read_session(&mut &bytes[..], dir.path(), 1, None);
        end_session(session, false, dir.path(), 1, None);

        let log = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
        assert!(
            log.lines()
                .any(|line| line.contains("a session message arrived before the session's header")),
            "{log}"
        );
        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert!(
            !journal.contains("a step before the header"),
            "the step before the header reached the folder: {journal}"
        );
        assert_eq!(journal.matches("[[step]]").count(), 1, "{journal}");
        assert!(journal.contains("how = \"closed\""), "{journal}");
    }

    // A closing message before any header — `Closed` or `Crashed`, the
    // warning covers both — is one Warn line in the slot's log, and with
    // no header there is no writer, so no folder.
    #[test]
    fn a_closing_message_before_the_header_warns_and_writes_nothing() {
        for (closing, name) in [
            (SessionFrame::Closed, "closed"),
            (SessionFrame::Crashed, "crashed"),
        ] {
            let dir = TestDir::new(&format!("session_early_close_{name}"));
            fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
            let mut bytes = Vec::new();
            write_frame(&mut bytes, &closing).unwrap();

            let session = read_session(&mut &bytes[..], dir.path(), 1, None);
            end_session(session, false, dir.path(), 1, None);

            let log = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
            assert!(
                log.lines()
                    .any(|line| line
                        .contains("the session's closing message arrived before its header")),
                "{log}"
            );
            assert!(!dir.path().join("sessions").exists(), "a folder was made");
        }
    }

    // A second header is one Warn line in the slot's log; the session keeps the
    // first — the folder is the first header's.
    #[test]
    fn a_second_header_warns_and_the_first_stands() {
        let dir = TestDir::new("session_second_header");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let mut bytes = Vec::new();
        for frame in [
            SessionFrame::Header(header(4242)),
            SessionFrame::Header(header(9999)),
            SessionFrame::Message(SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "a step".to_owned(),
                op: None,
            }),
        ] {
            write_frame(&mut bytes, &frame).unwrap();
        }

        let session = read_session(&mut &bytes[..], dir.path(), 1, None);
        end_session(session, false, dir.path(), 1, None);

        let log = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
        assert!(
            log.lines()
                .any(|line| line.contains("a second session header")),
            "{log}"
        );
        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert!(
            journal.contains("pid = 4242"),
            "the first header stands: {journal}"
        );
        assert_eq!(journal.matches("[[step]]").count(), 1, "{journal}");
    }

    // A frame that does not decode is one Warn line in the slot's log and ends the
    // reading: the header's writer still ends the session, `ended` without a
    // closing message.
    #[test]
    fn an_undecodable_frame_warns_and_ends_the_reading() {
        let dir = TestDir::new("session_bad_frame");
        fs::write(dir.path().join(LOG_FILE_NAME), "v\n").unwrap();
        let mut bytes = Vec::new();
        for frame in [
            SessionFrame::Header(header(4242)),
            SessionFrame::Message(SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "a step".to_owned(),
                op: None,
            }),
        ] {
            write_frame(&mut bytes, &frame).unwrap();
        }
        // A frame of an unknown kind byte, its body empty.
        bytes.extend_from_slice(&[99, 0, 0, 0, 0]);

        let session = read_session(&mut &bytes[..], dir.path(), 1, None);
        end_session(session, false, dir.path(), 1, None);

        let log = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
        assert!(
            log.lines()
                .any(|line| line.contains("a session frame could not be read")),
            "{log}"
        );
        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert!(journal.contains("how = \"ended\""), "{journal}");
    }
}
