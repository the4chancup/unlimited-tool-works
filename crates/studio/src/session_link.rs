//! The window's half of the session's link to its folder: `Recorder::send_to`
//! queues every step and kept file the recorder takes, and the thread `start`
//! spawns writes each into the session folder — to the crash monitor's standard
//! input as `SessionFrame`s when the monitor started, through the window's own
//! `SessionWriter` under `<data location>/sessions` when it did not. `close`
//! ends the queue; the thread then writes the session's closing state and the
//! folder reads `closed`.

use std::io::Write;
use std::path::Path;
use std::process::ChildStdin;
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
/// (`Recorder::send_to`) and the thread gets its receiver, `pipe` — the
/// monitor's standard input when `watch_crashes` gave one, `None` when it did
/// not — and `data_dir` for the window's own writer, the no-pipe case's
/// target. The session's header (`SessionHeader::for_this_window`) is what the
/// pipe carries first and what the writer's folder is named from.
pub(crate) fn start(recorder: &Recorder, pipe: Option<ChildStdin>, data_dir: &Path) -> SessionLink {
    let (tx, rx) = crossbeam_channel::bounded(QUEUE_CAPACITY);
    recorder.send_to(tx);
    let header = SessionHeader::for_this_window();
    let (how_tx, how_rx) = crossbeam_channel::bounded(1);
    let (done_tx, done) = crossbeam_channel::unbounded();
    let data_dir = data_dir.to_path_buf();
    let logger = log::logger();
    let thread = std::thread::spawn(move || {
        run(&rx, &how_rx, pipe, header, &data_dir, logger);
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

/// The thread's work: `pipe` gets the session's frames for the monitor;
/// `None` is the window's own writer under `data_dir`'s `sessions`.
fn run(
    rx: &Receiver<SessionMessage>,
    how: &Receiver<SessionEnd>,
    pipe: Option<ChildStdin>,
    header: SessionHeader,
    data_dir: &Path,
    logger: &dyn log::Log,
) {
    match pipe {
        Some(mut pipe) => send_session(rx, &mut pipe, header, how, logger),
        None => write_session(rx, header, data_dir, how, logger),
    }
}

/// The session written to the monitor's pipe as frames: the header first,
/// each queued message as it arrives, the closing frame last once `rx`'s
/// senders are gone — `Closed` or `Crashed`, `Ended` written as nothing,
/// the pipe's own end saying it. A failed write is one Warn line and the
/// queue is read no more: a pipe that broke says the monitor is gone, and
/// with the receiver dropped the queued leftovers and the recorder's later
/// sends meet the silent `Disconnected`, not its drop line.
fn send_session(
    rx: &Receiver<SessionMessage>,
    out: &mut impl Write,
    header: SessionHeader,
    how: &Receiver<SessionEnd>,
    logger: &dyn log::Log,
) {
    if !send_frame(out, &SessionFrame::Header(header), logger) {
        return;
    }
    while let Ok(message) = rx.recv() {
        if !send_frame(out, &SessionFrame::Message(message), logger) {
            return;
        }
    }
    match closing(how) {
        SessionEnd::Closed => {
            send_frame(out, &SessionFrame::Closed, logger);
        }
        SessionEnd::Crashed => {
            send_frame(out, &SessionFrame::Crashed, logger);
        }
        // `ended` is no frame: the pipe's own end says it.
        SessionEnd::Ended => {}
    }
}

/// `frame` sent to the monitor; `false` on a broken pipe, with the one Warn
/// line the break earns — a second write would meet the same end, so the
/// caller stops there.
fn send_frame(out: &mut impl Write, frame: &SessionFrame, logger: &dyn log::Log) -> bool {
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
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    use studio_core::repro::read_frame;

    use super::*;
    use crate::test_dir::TestDir;

    /// A `log::Log` collecting each record as `level|target|text` (the same
    /// capture `crash_dump`'s tests build), for the broken pipe's one Warn.
    #[derive(Default)]
    struct CaptureLog {
        records: Mutex<Vec<String>>,
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
                bytes: Arc::from(&b"INPUT_BYTES"[..]),
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
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let mut bytes = Vec::new();

        send_session(
            &rx,
            &mut bytes,
            header(4242),
            &how(SessionEnd::Closed),
            &logger,
        );

        let mut input = &bytes[..];
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
        let SessionFrame::Message(SessionMessage::Input { bytes, .. }) = frame else {
            panic!("an input frame follows the step")
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
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let mut bytes = Vec::new();

        send_session(
            &rx,
            &mut bytes,
            header(4242),
            &how(SessionEnd::Crashed),
            &logger,
        );

        let mut input = &bytes[..];
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
        let link = start(&recorder, None, dir.path());
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

    /// A writer whose every write fails: the broken pipe's far end.
    #[derive(Default)]
    struct FailingWrite {
        /// How many `write` calls it took; one is a write attempt, not a byte.
        writes: usize,
    }

    impl Write for FailingWrite {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            Err(io::Error::other("broken pipe"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // A broken pipe is one failed write and one Warn line, then the queue
    // is read no more: what it still holds stays in `rx`, its drop the
    // recorder's silent `Disconnected`.
    #[test]
    fn a_broken_pipe_warns_once_and_stops_reading_the_queue() {
        let rx = fed(messages());
        let logger = CaptureLog::default();
        let mut out = FailingWrite::default();
        let (_how_tx, how_rx) = crossbeam_channel::bounded(1);

        send_session(&rx, &mut out, header(4242), &how_rx, &logger);

        assert_eq!(out.writes, 1, "one write attempt, then the queue unread");
        let records = logger.records.lock().unwrap();
        assert_eq!(records.len(), 1, "{records:?}");
        assert!(records[0].starts_with("Warn|session_link|"), "{records:?}");
        assert!(
            matches!(rx.try_recv(), Ok(SessionMessage::Step { .. })),
            "the queue's first message is still held"
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
                bytes: Arc::from(&b"TOO_LONG"[..]),
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
