//! The window's half of the session's link to its folder: `Recorder::send_to`
//! queues every step and kept file the recorder takes, and the thread `start`
//! spawns writes each into the session folder — to the crash monitor's standard
//! input as `SessionFrame`s when the monitor started, through the window's own
//! `SessionWriter` under `<data location>/sessions` when it did not. A pipe
//! that breaks mid-session gets one new monitor from `restart`, which writes
//! on into the same folder; no new monitor, or a second break, gives the
//! folder to the window's own writer. `close` ends the queue; the thread then
//! writes the session's closing state and the folder reads `closed`.

use std::io::Write;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use studio_core::repro::{
    Recorder, SessionEnd, SessionFrame, SessionHeader, SessionMessage, SessionWriter, write_frame,
};

/// The session queue's bound: the window's bursts of steps — an import's,
/// a multi-row paste's — stay far under it, so a full queue means the
/// writer stalled (a monitor that reads nothing) rather than fell behind.
/// Past it a dropped message costs the session less than a window made to
/// wait on the queue.
const QUEUE_CAPACITY: usize = 256;

/// `close`'s wait on the sender thread: the closing state normally lands at
/// once, but a monitor that stopped reading can hold the thread inside a pipe
/// write, and the window's close cannot wait on that.
const CLOSE_WAIT: Duration = Duration::from_secs(5);

/// The session link the window holds until `close`. The thread owns the
/// queue's receiver and, when the monitor started, its pipe.
pub(crate) struct SessionLink {
    /// The queue-draining thread.
    thread: JoinHandle<()>,
    /// The thread's end report: `JoinHandle` offers no timed wait, so the
    /// thread reports itself over this channel instead.
    done: Receiver<()>,
    /// The channel `close` sends the session's ending on: one slot, read by
    /// the thread once the queue ends — the end is its cue to look.
    how_tx: Sender<SessionEnd>,
    /// The recorder `start` was given, kept so `close` detaches the same
    /// sink the thread's queue feeds from.
    recorder: Recorder,
}

/// Starts the session's sender thread: `recorder` gets the queue's sender
/// (`Recorder::send_to`) and the thread gets its receiver, `target` — the
/// monitor's standard input when `watch_crashes` gave one, `None` when it did
/// not — `restart`, which gives the one new monitor's pipe a break earns
/// (asked once, and with `target` `None` never), and `data_dir` for the
/// window's own writer, the no-pipe case's target and the fallback the
/// second break gets. `header` is what the pipe carries first and what the
/// writer's folder is named from; the caller makes it, so the folder's name
/// is the caller's to hand the shell too.
pub(crate) fn start(
    recorder: &Recorder,
    target: Option<Box<dyn Write + Send>>,
    restart: Box<dyn FnMut() -> Option<Box<dyn Write + Send>> + Send>,
    data_dir: &Path,
    header: SessionHeader,
) -> SessionLink {
    let (tx, rx) = crossbeam_channel::bounded(QUEUE_CAPACITY);
    recorder.send_to(tx);
    let (how_tx, how_rx) = crossbeam_channel::bounded(1);
    let (done_tx, done) = crossbeam_channel::unbounded();
    let data_dir = data_dir.to_path_buf();
    let logger = log::logger();
    let thread = std::thread::spawn(move || {
        run(&rx, &how_rx, target, restart, header, &data_dir, logger);
        // An unlistened report is no error: `close` may have timed out first.
        if let Err(error) = done_tx.send(()) {
            log::debug!("the session link's end was not waited for: {error}");
        }
    });
    SessionLink {
        thread,
        done,
        how_tx,
        recorder: recorder.clone(),
    }
}

impl SessionLink {
    /// Ends the session as `how` — `Closed`, or `Crashed` after a panic the
    /// window caught — sent ahead of the recorder's sink going, so the
    /// queue's end finds it waiting; the thread then writes the closing
    /// state (`Closed`/`Crashed` on the pipe, `end(how)` on the window's own
    /// writer) and reports `done`, waited on for `CLOSE_WAIT`. A monitor
    /// that stopped reading can hold the thread inside a write; the
    /// window's close waits on it no longer than that, and a timed-out
    /// thread is left for the process's end.
    pub(crate) fn close(self, how: SessionEnd) {
        // `how` goes first: the queue's end is the thread's cue to read it,
        // so it must be waiting when the recorder's sink drops.
        if self.how_tx.send(how).is_err() {
            log::debug!("the session's sender thread was already gone");
        }
        self.recorder.close_sink();
        match self.done.recv_timeout(CLOSE_WAIT) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => {
                log::warn!("the session's sender thread did not finish within {CLOSE_WAIT:?}");
            }
            // The thread ended without reporting — a panic: joining
            // retrieves it.
            Err(RecvTimeoutError::Disconnected) => {
                if let Err(panic) = self.thread.join() {
                    let text = panic
                        .downcast_ref::<&'static str>()
                        .copied()
                        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                        .unwrap_or("a panic with no message");
                    log::warn!("the session's sender thread panicked: {text}");
                }
            }
        }
    }
}

/// `text` logged at `Warn` through `logger`, the way `watch_crashes` reports
/// its failures — the sender thread's lines go through it so a test can
/// count them.
fn warn(logger: &dyn log::Log, text: String) {
    logger.log(
        &log::Record::builder()
            .level(log::Level::Warn)
            .target("session_link")
            .args(format_args!("{text}"))
            .build(),
    );
}

/// The closing state `close` sent ahead of the queue's end; `Closed` when the
/// queue ended with nothing sent — `main` always sends first, so that case
/// is a default, never a reached one.
fn closing(how: &Receiver<SessionEnd>) -> SessionEnd {
    how.try_recv().unwrap_or(SessionEnd::Closed)
}

/// The thread's work: `target` gets the session's frames for the monitor,
/// with `restart` the one new monitor a break earns; `None` is the window's
/// own writer under `data_dir`'s `sessions`, the run with no monitor.
fn run(
    rx: &Receiver<SessionMessage>,
    how: &Receiver<SessionEnd>,
    target: Option<Box<dyn Write + Send>>,
    restart: Box<dyn FnMut() -> Option<Box<dyn Write + Send>> + Send>,
    header: SessionHeader,
    data_dir: &Path,
    logger: &dyn log::Log,
) {
    match target {
        Some(pipe) => send_session(rx, pipe, restart, header, data_dir, how, logger),
        None => write_session(rx, header, data_dir, how, logger),
    }
}

/// Where a session frame goes: the monitor's pipe while it holds, the
/// window's own `SessionWriter` under `sessions` after the second break or
/// no new monitor.
enum Target {
    /// The monitor's standard input; a frame goes as `write_frame`'s bytes.
    Pipe(Box<dyn Write + Send>),
    /// The window's own writer, which adopts the folder the monitor left.
    Own(SessionWriter),
}

impl Target {
    /// `frame` delivered: on the pipe as a frame (`false` is a broken pipe,
    /// already warned), on the writer as its row or `[end]` — a failed write
    /// warns and loses the one message while the session reads on, as
    /// `write_session` does (`true`). A header reaches the writer through its
    /// `new`, so resending one writes nothing.
    fn send(&mut self, frame: &SessionFrame, logger: &dyn log::Log) -> bool {
        match self {
            Target::Pipe(pipe) => send_frame(&mut **pipe, frame, logger),
            Target::Own(writer) => {
                match frame {
                    SessionFrame::Message(message) => {
                        if let Err(error) = writer.write(message) {
                            warn(
                                logger,
                                format!("a session message could not be written: {error:#}"),
                            );
                        }
                    }
                    SessionFrame::Closed | SessionFrame::Crashed => {
                        let how = if matches!(frame, SessionFrame::Crashed) {
                            SessionEnd::Crashed
                        } else {
                            SessionEnd::Closed
                        };
                        if let Err(error) = writer.end(how) {
                            warn(
                                logger,
                                format!("the session's end could not be written: {error:#}"),
                            );
                        }
                    }
                    SessionFrame::Header(_) => {}
                }
                true
            }
        }
    }
}

/// `frame` sent on `target`, a broken pipe moving it once: to `restart`'s
/// new monitor — `header` sent again on its pipe first, since it learns the
/// session afresh — or, with none coming back or its pipe breaking too, to
/// the window's own writer under `data_dir`'s `sessions`, which adopts the
/// folder the dead monitor left. The frame is then sent on the new target.
/// `restarted` keeps `restart` to the one new monitor.
fn give(
    target: &mut Target,
    frame: &SessionFrame,
    header: &SessionHeader,
    restarted: &mut bool,
    restart: &mut impl FnMut() -> Option<Box<dyn Write + Send>>,
    data_dir: &Path,
    logger: &dyn log::Log,
) {
    while !target.send(frame, logger) {
        if !*restarted {
            *restarted = true;
            if let Some(pipe) = restart() {
                *target = Target::Pipe(pipe);
                if matches!(frame, SessionFrame::Header(_))
                    || target.send(&SessionFrame::Header(header.clone()), logger)
                {
                    continue;
                }
            }
        }
        *target = Target::Own(SessionWriter::new(
            &data_dir.join("sessions"),
            header.clone(),
        ));
    }
}

/// The session written to the monitor's pipe as frames: the header first,
/// each queued message as it arrives, the closing frame last once `rx`'s
/// senders are gone — `Closed` or `Crashed`, `Ended` written as nothing,
/// the pipe's own end saying it. A failed write is one Warn line, then
/// `restart` once: a new monitor's pipe gets the header again and the frame
/// the break interrupted, and the session goes on; `None` back, or the new
/// pipe breaking too, moves the session to the window's own writer under
/// `data_dir`'s `sessions`, interrupted frame first — it adopts the folder
/// the dead monitor left, so the folder ends as one session.
fn send_session(
    rx: &Receiver<SessionMessage>,
    pipe: Box<dyn Write + Send>,
    mut restart: impl FnMut() -> Option<Box<dyn Write + Send>>,
    header: SessionHeader,
    data_dir: &Path,
    how: &Receiver<SessionEnd>,
    logger: &dyn log::Log,
) {
    let mut target = Target::Pipe(pipe);
    let mut restarted = false;
    give(
        &mut target,
        &SessionFrame::Header(header.clone()),
        &header,
        &mut restarted,
        &mut restart,
        data_dir,
        logger,
    );
    while let Ok(message) = rx.recv() {
        give(
            &mut target,
            &SessionFrame::Message(message),
            &header,
            &mut restarted,
            &mut restart,
            data_dir,
            logger,
        );
    }
    match closing(how) {
        SessionEnd::Closed => give(
            &mut target,
            &SessionFrame::Closed,
            &header,
            &mut restarted,
            &mut restart,
            data_dir,
            logger,
        ),
        SessionEnd::Crashed => give(
            &mut target,
            &SessionFrame::Crashed,
            &header,
            &mut restarted,
            &mut restart,
            data_dir,
            logger,
        ),
        // `ended` is no frame on the pipe — its end says it — but the own
        // writer's `[end]` says it too.
        SessionEnd::Ended => {
            if let Target::Own(writer) = &mut target
                && let Err(error) = writer.end(SessionEnd::Ended)
            {
                warn(
                    logger,
                    format!("the session's end could not be written: {error:#}"),
                );
            }
        }
    }
}

/// `frame` sent to the monitor; `false` on a broken pipe, with the one Warn
/// line the break earns — a second write would meet the same end, so the
/// caller moves the session on.
fn send_frame(
    out: &mut (impl Write + ?Sized),
    frame: &SessionFrame,
    logger: &dyn log::Log,
) -> bool {
    match write_frame(out, frame) {
        Ok(()) => true,
        Err(error) => {
            warn(
                logger,
                format!("the session's link to the crash monitor broke: {error}"),
            );
            false
        }
    }
}

/// The session written by the window's own `SessionWriter` under `data_dir`'s
/// `sessions`, for a window whose monitor did not start: the folder reads as
/// the monitor would have written it — a failed write is one Warn line and
/// the session reads on, so a bad message loses itself and nothing after it.
fn write_session(
    rx: &Receiver<SessionMessage>,
    header: SessionHeader,
    data_dir: &Path,
    how: &Receiver<SessionEnd>,
    logger: &dyn log::Log,
) {
    let mut writer = SessionWriter::new(&data_dir.join("sessions"), header);
    while let Ok(message) = rx.recv() {
        if let Err(error) = writer.write(&message) {
            warn(
                logger,
                format!("a session message could not be written: {error:#}"),
            );
        }
    }
    if let Err(error) = writer.end(closing(how)) {
        warn(
            logger,
            format!("the session's end could not be written: {error:#}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant, SystemTime};

    use studio_core::repro::{SessionFile, read_frame};

    use super::*;
    use crate::test_dir::TestDir;

    /// A `log::Log` collecting each record as `level|target|text` (the same
    /// capture `crash_dump`'s tests build), for the broken pipe's one Warn.
    /// The list is an `Arc` so a test's `restart` closure can log into the
    /// same list the logger handed to the guard writes.
    #[derive(Default)]
    struct CaptureLog {
        records: Arc<Mutex<Vec<String>>>,
    }

    impl log::Log for CaptureLog {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            self.records.lock().unwrap().push(format!(
                "{:?}|{}|{}",
                record.level(),
                record.target(),
                record.args()
            ));
        }

        fn flush(&self) {}
    }

    /// A `Write` over a shared byte buffer, so a test reads what the
    /// sender's thread wrote after `send_session` took the writer.
    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// The buffer's frames read back in order.
    fn read_frames(buffer: &SharedBuffer) -> Vec<SessionFrame> {
        let bytes = buffer.0.lock().unwrap().clone();
        let mut input = &bytes[..];
        let mut frames = Vec::new();
        while let Some(frame) = read_frame(&mut input).unwrap() {
            frames.push(frame);
        }
        frames
    }

    /// A session header: `started` at the epoch, built through `SystemTime`
    /// (jiff is not a `studio` dependency).
    fn header(pid: u32) -> SessionHeader {
        SessionHeader {
            version: "v-test".to_owned(),
            os: "test-os".to_owned(),
            started: SystemTime::UNIX_EPOCH.try_into().unwrap(),
            pid,
        }
    }

    /// A stub session's messages: a step and a kept input.
    fn messages() -> Vec<SessionMessage> {
        vec![
            SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "opened C:/Data/in/EDIT00000000 (10 bytes)".to_owned(),
                op: None,
            },
            SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"INPUT_BYTES"[..])),
            },
        ]
    }

    /// `rx` fed `messages` and closed from the sender's side, as the
    /// recorder's sink going leaves it.
    fn fed(messages: Vec<SessionMessage>) -> Receiver<SessionMessage> {
        let (tx, rx) = crossbeam_channel::unbounded();
        for message in messages {
            tx.send(message).unwrap();
        }
        drop(tx);
        rx
    }

    /// A `how` channel holding `end`, as `close` leaves it for the thread.
    fn how(end: SessionEnd) -> Receiver<SessionEnd> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        tx.send(end).unwrap();
        rx
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

    // With no monitor, the window's own writer writes the same folder the
    // monitor would: the header's lines, the step and the input with its
    // file, and `[end]` `closed`.
    // ST-REP-10
    #[test]
    fn without_a_monitor_the_windows_own_writer_writes_the_folder() {
        let dir = TestDir::new("session_own_writer");
        let rx = fed(messages());
        let logger = CaptureLog::default();

        write_session(
            &rx,
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert!(journal.contains("version = \"v-test\""), "{journal}");
        assert!(journal.contains("os = \"test-os\""), "{journal}");
        assert!(journal.contains("pid = 4242"), "{journal}");
        assert_eq!(journal.matches("[[step]]").count(), 1, "{journal}");
        assert_eq!(journal.matches("[[input]]").count(), 1, "{journal}");
        assert!(
            journal.contains("opened C:/Data/in/EDIT00000000"),
            "{journal}"
        );
        assert_eq!(
            fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
            b"INPUT_BYTES"
        );
        assert!(journal.contains("how = \"closed\""), "{journal}");
    }

    // The pipe's bytes read back with `read_frame` as the header, the
    // queued messages in order, then `Closed`.
    #[test]
    fn the_pipe_carries_the_header_the_messages_and_the_close() {
        let dir = TestDir::new("session_pipe_frames");
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let buffer = SharedBuffer::default();

        send_session(
            &rx,
            Box::new(buffer.clone()),
            Box::new(|| None),
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        let mut input = &buffer.0.lock().unwrap()[..];
        let frame = read_frame(&mut input).unwrap().expect("the header");
        let SessionFrame::Header(read) = frame else {
            panic!("the header comes first")
        };
        assert_eq!(read.pid, 4242);
        assert_eq!(read.version, "v-test");
        let frame = read_frame(&mut input).unwrap().expect("the step");
        let SessionFrame::Message(SessionMessage::Step { what, .. }) = frame else {
            panic!("a step frame follows the header")
        };
        assert_eq!(what, "opened C:/Data/in/EDIT00000000 (10 bytes)");
        let frame = read_frame(&mut input).unwrap().expect("the input");
        let SessionFrame::Message(SessionMessage::Input { file, .. }) = frame else {
            panic!("an input frame follows the step")
        };
        let SessionFile::Raw(bytes) = file else {
            panic!("the stub's input is raw")
        };
        assert_eq!(&*bytes, b"INPUT_BYTES");
        assert!(
            matches!(read_frame(&mut input).unwrap(), Some(SessionFrame::Closed)),
            "the closing frame is last"
        );
        assert!(
            read_frame(&mut input).unwrap().is_none(),
            "nothing follows the close"
        );
    }

    // A `Crashed` close travels the pipe the way `Closed` does: the
    // header, the messages, then the crashed closing frame.
    // ST-REP-10
    #[test]
    fn the_pipe_carries_the_crashed_close() {
        let dir = TestDir::new("session_pipe_crashed");
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let buffer = SharedBuffer::default();

        send_session(
            &rx,
            Box::new(buffer.clone()),
            Box::new(|| None),
            header(4242),
            dir.path(),
            &how(SessionEnd::Crashed),
            &logger,
        );

        let mut input = &buffer.0.lock().unwrap()[..];
        for expected in 0..3 {
            assert!(
                read_frame(&mut input).unwrap().is_some(),
                "frame {expected} is written"
            );
        }
        assert!(
            matches!(read_frame(&mut input).unwrap(), Some(SessionFrame::Crashed)),
            "the closing frame is `Crashed`"
        );
    }

    // The window's own writer takes the crashed close too: `[end]` reads
    // `crashed` where a clean close reads `closed`.
    // ST-REP-10
    #[test]
    fn the_own_writer_writes_the_crashed_end() {
        let dir = TestDir::new("session_own_writer_crashed");
        let rx = fed(messages());
        let logger = CaptureLog::default();

        write_session(
            &rx,
            header(4242),
            dir.path(),
            &how(SessionEnd::Crashed),
            &logger,
        );

        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert!(journal.contains("how = \"crashed\""), "{journal}");
    }

    // `start` and `close` end to end, no monitor: a step and a kept input
    // recorded through the recorder land in the folder, ended `closed`.
    // ST-REP-10
    #[test]
    fn the_links_close_writes_the_folder_through_the_real_wiring() {
        let dir = TestDir::new("session_link_end_to_end");
        let recorder = Recorder::default();
        let link = start(&recorder, None, Box::new(|| None), dir.path(), header(4242));
        recorder.step("stub", "a step through the real queue");
        recorder.keep_input(
            "stub",
            Path::new("C:/Data/in/EDIT00000000"),
            Arc::from(&b"IN_BYTES"[..]),
        );

        link.close(SessionEnd::Closed);

        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert_eq!(journal.matches("[[step]]").count(), 1, "{journal}");
        assert!(
            journal.contains("a step through the real queue"),
            "{journal}"
        );
        assert_eq!(journal.matches("[[input]]").count(), 1, "{journal}");
        assert_eq!(
            fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
            b"IN_BYTES"
        );
        assert!(journal.contains("how = \"closed\""), "{journal}");
    }

    /// A writer that lets `ok` `write` calls through, then fails every
    /// one: the pipe at a break. `write_frame` is three calls a frame —
    /// the kind byte, the length and the body — so `ok` counts frames
    /// as `3 * frames`.
    struct FailingWrite {
        /// `write` calls that succeed before the break.
        ok: usize,
        /// How many `write` calls it took; one is an attempt, not a byte.
        writes: usize,
    }

    impl FailingWrite {
        /// One that fails from its `frames`'s first write on (three
        /// calls a frame make it through).
        fn after(frames: usize) -> FailingWrite {
            FailingWrite {
                ok: frames * 3,
                writes: 0,
            }
        }
    }

    impl Write for FailingWrite {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            if self.writes > self.ok {
                return Err(io::Error::other("broken pipe"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // The link's close runs inside the panic guard, before the session's
    // closing line: a pipe that broke while the window sat idle is found by
    // that close — the header's writes passed, and the closing frame's are
    // the break — and the new monitor's line must land ahead of the closing
    // line, the log's last, which the next start reads as the session's end.
    #[test]
    fn the_links_close_runs_before_the_closing_line() {
        let dir = TestDir::new("session_link_close_order");
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let logger = CaptureLog {
            records: events.clone(),
        };
        let restarts = CaptureLog {
            records: events.clone(),
        };
        let recorder = Recorder::default();
        let link = start(
            &recorder,
            Some(Box::new(FailingWrite::after(1))),
            Box::new(move || {
                log::Log::log(
                    &restarts,
                    &log::Record::builder()
                        .level(log::Level::Info)
                        .target("crash_dump")
                        .args(format_args!(
                            "a new crash monitor was started after the session's pipe broke"
                        ))
                        .build(),
                );
                None
            }),
            dir.path(),
            header(4242),
        );

        let code = crate::gui_log::guard_gui(
            || ExitCode::SUCCESS,
            None,
            |_| {},
            move |code| link.close(crate::session_end(code)),
            &logger,
        );

        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(
            events[0].contains("a new crash monitor was started"),
            "{events:?}"
        );
        assert_eq!(
            events[1], "Info|session|the window closed normally",
            "{events:?}"
        );
    }

    /// The stalled-close watchdog's bound: three times `close`'s own wait,
    /// so a machine hiccup never trips it and a hung `close` always does.
    const WATCHDOG: Duration = Duration::from_secs(15);

    /// A `Write` whose `write` waits on the test's release: the monitor
    /// that stopped reading, holding the sender thread inside a pipe write.
    /// The release, or the last sender's drop, ends the wait as the broken
    /// pipe the stall would have read as.
    struct BlockingWrite {
        released: Receiver<()>,
    }

    impl Write for BlockingWrite {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            match self.released.recv() {
                Ok(()) => Err(io::Error::other("broken pipe")),
                Err(error) => Err(io::Error::other(error)),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // A writer stalled mid-session does not hold the close: the thread sits
    // inside the header's write, a recorded message queues behind it, and
    // `close` waits the thread out no longer than `CLOSE_WAIT`. The watchdog
    // bounds the test rather than letting a hung `close` hang it: past
    // `WATCHDOG` it releases the writer so the wait can end, and the
    // `expired` flag fails the run.
    #[test]
    fn a_stalled_writer_does_not_hold_the_close() {
        let dir = TestDir::new("session_link_stalled");
        let recorder = Recorder::default();
        let (release_tx, release_rx) = crossbeam_channel::bounded::<()>(1);
        let (finished_tx, finished_rx) = crossbeam_channel::bounded::<()>(1);
        let expired = Arc::new(AtomicBool::new(false));
        let watchdog = std::thread::spawn({
            let expired = expired.clone();
            let release_tx = release_tx.clone();
            move || {
                if finished_rx.recv_timeout(WATCHDOG).is_err() {
                    expired.store(true, Ordering::Relaxed);
                    release_tx.send(()).unwrap_or(());
                }
            }
        });
        let link = start(
            &recorder,
            Some(Box::new(BlockingWrite {
                released: release_rx,
            })),
            Box::new(|| None),
            dir.path(),
            header(4242),
        );
        recorder.step("stub", "a step while the writer is stalled");

        let started = Instant::now();
        link.close(SessionEnd::Closed);
        let elapsed = started.elapsed();
        // `close` is back inside the bound: the watchdog goes home, then the
        // writer's release unwinds the thread it stalled.
        finished_tx.send(()).unwrap_or(());
        watchdog.join().unwrap();
        release_tx.send(()).unwrap_or(());

        assert!(
            !expired.load(Ordering::Relaxed),
            "close did not return within {WATCHDOG:?}"
        );
        assert!(
            elapsed >= CLOSE_WAIT,
            "the stalled writer did not hold the close: {elapsed:?}"
        );
        assert!(
            elapsed < CLOSE_WAIT + Duration::from_secs(5),
            "the close outran its bound: {elapsed:?}"
        );
    }

    // A broken pipe is one Warn line, then `restart` once: the new pipe
    // reads back as the header again, the message the break interrupted,
    // the rest and `Closed`.
    // ST-REP-10
    #[test]
    fn a_broken_pipe_gets_one_new_monitor_writing_on() {
        let dir = TestDir::new("session_link_restart");
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let buffer = SharedBuffer::default();
        let new_pipe = buffer.clone();
        let restarts = Cell::new(0);

        send_session(
            &rx,
            Box::new(FailingWrite::after(1)),
            || {
                restarts.set(restarts.get() + 1);
                Some(Box::new(new_pipe.clone()) as Box<dyn Write + Send>)
            },
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        assert_eq!(restarts.get(), 1, "the one restart the break earns");
        let frames = read_frames(&buffer);
        let [
            SessionFrame::Header(_),
            SessionFrame::Message(SessionMessage::Step { .. }),
            SessionFrame::Message(SessionMessage::Input { .. }),
            SessionFrame::Closed,
        ] = frames[..]
        else {
            panic!(
                "the new pipe's frames are not the header, the interrupted message, the rest and the close"
            )
        };
        let records = logger.records.lock().unwrap();
        assert_eq!(records.len(), 1, "{records:?}");
        assert!(records[0].starts_with("Warn|session_link|"), "{records:?}");
        assert!(
            records[0].contains("link to the crash monitor broke"),
            "{records:?}"
        );
        assert!(
            !dir.path().join("sessions").exists(),
            "the session stayed on the pipe: no folder was made"
        );
    }

    /// `header`'s folder under `dir`'s `sessions` holding `written`'s
    /// rows, as a dead monitor's writer left it.
    fn dead_folder(dir: &TestDir, header: SessionHeader, written: Vec<SessionMessage>) {
        let mut writer = SessionWriter::new(&dir.path().join("sessions"), header);
        for message in &written {
            writer.write(message).unwrap();
        }
    }

    /// The stub step, spelled a second time for a dead writer's folder.
    fn the_step() -> SessionMessage {
        SessionMessage::Step {
            seconds: 0.5,
            tool: "stub".to_owned(),
            what: "opened C:/Data/in/EDIT00000000 (10 bytes)".to_owned(),
            op: None,
        }
    }

    // `restart` gives no new monitor: the window's own writer adopts the
    // folder the dead monitor wrote — the interrupted message and every
    // later one land in it once, in order, ended `closed`.
    // ST-REP-10
    #[test]
    fn with_no_new_monitor_the_own_writer_writes_on_in_the_same_folder() {
        let dir = TestDir::new("session_link_no_restart");
        dead_folder(&dir, header(4242), vec![the_step()]);
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let restarts = Cell::new(0);

        send_session(
            &rx,
            Box::new(FailingWrite::after(2)),
            || {
                restarts.set(restarts.get() + 1);
                None
            },
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        assert_eq!(restarts.get(), 1, "`restart` was asked once");
        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert_eq!(journal.matches("[[step]]").count(), 1, "{journal}");
        assert_eq!(journal.matches("[[input]]").count(), 1, "{journal}");
        assert_eq!(
            fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
            b"INPUT_BYTES",
            "the interrupted message, written by the own writer"
        );
        assert!(journal.contains("how = \"closed\""), "{journal}");
        assert_eq!(
            logger.records.lock().unwrap().len(),
            1,
            "the break's one Warn"
        );
    }

    // The new monitor's pipe breaking too hands the session to the
    // window's own writer: `restart` is not asked again, and the frame
    // the second break interrupted and the rest land in the folder,
    // ended `closed`.
    // ST-REP-10
    #[test]
    fn a_second_break_gives_the_folder_to_the_own_writer() {
        let dir = TestDir::new("session_link_second_break");
        dead_folder(&dir, header(4242), vec![the_step()]);
        let messages = vec![
            the_step(),
            SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"INPUT_BYTES"[..])),
            },
            SessionMessage::Step {
                seconds: 1.0,
                tool: "stub".to_owned(),
                what: "the step after the break".to_owned(),
                op: None,
            },
        ];
        let rx = fed(messages);
        let logger = CaptureLog::default();
        let restarts = Cell::new(0);
        let spare = SharedBuffer::default();
        let spare_pipe = spare.clone();

        send_session(
            &rx,
            Box::new(FailingWrite::after(2)),
            || {
                restarts.set(restarts.get() + 1);
                // First call: the new monitor, broken on its own first
                // message. A second call — a `restart` on every failed
                // write — would hand the session's rest to `spare`,
                // and the folder's assertions would see it.
                Some(match restarts.get() {
                    1 => Box::new(FailingWrite::after(1)) as Box<dyn Write + Send>,
                    _ => Box::new(spare_pipe.clone()) as Box<dyn Write + Send>,
                })
            },
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        assert_eq!(restarts.get(), 1, "`restart` was asked the once");
        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert_eq!(
            journal.matches("[[step]]").count(),
            2,
            "the dead writer's step and the one after the break: {journal}"
        );
        assert_eq!(journal.matches("[[input]]").count(), 1, "{journal}");
        assert_eq!(
            fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
            b"INPUT_BYTES",
            "the second break's interrupted message"
        );
        assert!(journal.contains("how = \"closed\""), "{journal}");
        assert!(
            spare.0.lock().unwrap().is_empty(),
            "no second restart wrote anything"
        );
        assert_eq!(
            logger.records.lock().unwrap().len(),
            2,
            "one Warn per break"
        );
    }

    // A break on the closing frame itself, with no new monitor: the own
    // writer adopts the folder the dead monitor left and ends it — its
    // first act on the folder is the `[end]`.
    // ST-REP-10
    #[test]
    fn a_break_on_the_close_ends_the_adopted_folder_closed() {
        let dir = TestDir::new("session_link_close_break");
        dead_folder(&dir, header(4242), messages());
        let rx = fed(Vec::new());
        let logger = CaptureLog::default();

        send_session(
            &rx,
            Box::new(FailingWrite::after(1)),
            || None,
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert_eq!(
            journal.matches("\n[end]\n").count(),
            1,
            "the one [end]: {journal}"
        );
        assert!(journal.contains("how = \"closed\""), "{journal}");
        assert_eq!(
            journal.matches("[[step]]").count(),
            1,
            "the dead monitor's rows stand: {journal}"
        );
    }

    // A message the folder cannot take — a kept input whose file name
    // (300 bytes) no filesystem accepts — is one Warn line and the
    // session reads on: the step after it lands, and `[end]` `closed`.
    #[test]
    fn a_failed_message_write_warns_and_the_session_reads_on() {
        let dir = TestDir::new("session_write_error");
        let rx = fed(vec![
            SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "the first step".to_owned(),
                op: None,
            },
            SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/").join("n".repeat(300)),
                file: SessionFile::Raw(Arc::from(&b"TOO_LONG"[..])),
            },
            SessionMessage::Step {
                seconds: 0.6,
                tool: "stub".to_owned(),
                what: "the step after the bad one".to_owned(),
                op: None,
            },
        ]);
        let logger = CaptureLog::default();

        write_session(
            &rx,
            header(4242),
            dir.path(),
            &how(SessionEnd::Closed),
            &logger,
        );

        let folder = session_folder(&dir);
        let journal = fs::read_to_string(folder.join("journal.toml")).unwrap();
        assert_eq!(journal.matches("[[step]]").count(), 2, "{journal}");
        assert!(
            journal.contains("the step after the bad one"),
            "the session read on past the failed write: {journal}"
        );
        assert_eq!(
            journal.matches("[[input]]").count(),
            0,
            "the failed input left no row: {journal}"
        );
        assert!(journal.contains("how = \"closed\""), "{journal}");
        let records = logger.records.lock().unwrap();
        assert_eq!(
            records.len(),
            1,
            "one Warn for the one bad write: {records:?}"
        );
    }
}
