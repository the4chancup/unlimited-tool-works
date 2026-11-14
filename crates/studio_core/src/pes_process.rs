//! Which PES games are running now (`spec/core/architecture.md` "Crate structure", the
//! running-PES probe): the Save
//! editor's writing commands refuse while one is, and the GUI's version selector polls
//! it on a timer.

use std::path::{Path, PathBuf};

use pes_version::PesVersion;

use crate::tool::ExeCheck;

/// A PES game running now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningPes {
    /// Which game it is, matched from its executable's name.
    pub version: PesVersion,
    /// The executable's full path, `None` where the process would not say (one of another
    /// user's session, or a protected process).
    pub exe: Option<PathBuf>,
}

/// The PES games running now (`spec/core/architecture.md` "Crate structure", the running-PES
/// probe).
pub fn running_pes() -> Vec<RunningPes> {
    probe()
}

/// The version a `PES20NN.exe` name says it is: `PES2015.exe` to `PES2021.exe` in
/// any letter case, the names the games ship with. Anything else (`PES2014.exe`,
/// `PES20021.exe`, `PES2021.exe.bak`, `pes2021`, `xPES2021.exe`, `pes21.exe`) is no match.
/// The one rule for an exe's name: the running-PES probe and the sidebar's exe check both
/// read through it.
pub(crate) fn version_of_exe(name: &str) -> Option<PesVersion> {
    let lower = name.to_ascii_lowercase();
    let year: u16 = lower
        .strip_prefix("pes")?
        .strip_suffix(".exe")?
        .parse()
        .ok()?;
    // `from_number` also accepts the short `pes21.exe`, which is not a shipped name.
    PesVersion::from_number(year).filter(|version| version.year() == year)
}

/// One directory listing of `folder` for `PES20NN.exe` names, in any letter case:
/// `version`'s own exe wins when several are present (ST-SIDE-03). Only files count:
/// the condition mirrors the compiler's preflight (`is_file` on the exe's path). The
/// shell's one exe check: the sidebar colors its selector by it, and a tool reads the
/// verdict through `ToolContext`.
pub(crate) fn check_pes_folder(version: PesVersion, folder: &Path) -> ExeCheck {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(_) => return ExeCheck::NoFolder,
    };
    let mut other = None;
    for entry in entries.flatten() {
        // `Path::is_file` follows a link the same way the compiler's preflight does.
        if !entry.path().is_file() {
            continue;
        }
        let Some(found) = entry.file_name().to_str().and_then(version_of_exe) else {
            continue;
        };
        if found == version {
            return ExeCheck::Found;
        }
        if other.is_none() {
            other = Some(found);
        }
    }
    other.map_or(ExeCheck::NoExe, ExeCheck::OtherVersion)
}

/// The game a `/proc/<pid>/cmdline` buffer says its process is, or `None`: the first
/// NUL-separated argument's last segment after a `\` or `/` is the exe name (a Windows
/// path under Wine or Proton), and the argument itself is its `exe`. `#[cfg(test)]`
/// lets the parse run on every platform.
#[cfg(any(target_os = "linux", test))]
fn exe_of_cmdline(bytes: &[u8]) -> Option<(PesVersion, PathBuf)> {
    let argument = bytes.split(|byte| *byte == 0).next()?;
    let argument = String::from_utf8_lossy(argument);
    let name = argument.rsplit(['\\', '/']).next()?;
    let version = version_of_exe(name)?;
    Some((version, PathBuf::from(argument.as_ref())))
}

/// Windows: every process the Toolhelp snapshot lists, its id and its exe file name.
/// A process the snapshot cannot read is skipped.
#[cfg(windows)]
fn processes() -> Vec<(u32, String)> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    // SAFETY: asks only for a process-list snapshot; the handle is closed before this
    // function returns.
    let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::debug!("the process snapshot failed: {error}");
            return Vec::new();
        }
    };
    let mut processes = Vec::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: `snapshot` is a live snapshot handle and `entry` a valid out-entry with
    // its `dwSize` set.
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while more {
        let end = entry
            .szExeFile
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(entry.szExeFile.len());
        processes.push((
            entry.th32ProcessID,
            String::from_utf16_lossy(&entry.szExeFile[..end]),
        ));
        // SAFETY: the same live snapshot handle and out-entry as `Process32FirstW`.
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: `snapshot` is an owned handle this function created and is done with.
    if let Err(error) = unsafe { CloseHandle(snapshot) } {
        log::debug!("closing the process snapshot failed: {error}");
    }
    processes
}

/// Windows: the games among `processes()`, each match's image path asked of the
/// process itself.
#[cfg(windows)]
fn probe() -> Vec<RunningPes> {
    processes()
        .into_iter()
        .filter_map(|(id, name)| {
            version_of_exe(&name).map(|version| RunningPes {
                version,
                exe: image_path(id),
            })
        })
        .collect()
}

/// The full image path of `process_id`, `None` when the process refuses the query. The
/// limited right is granted on an elevated process too; another user's or a protected one
/// can refuse it.
#[cfg(windows)]
fn image_path(process_id: u32) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::core::PWSTR;

    // SAFETY: asks for the query-limited right only; the handle is closed before this
    // function returns.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
        .inspect_err(|error| log::debug!("process {process_id} refused a query: {error}"))
        .ok()?;
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    // SAFETY: `process` is a live handle; `buffer` is writable for `len` UTF-16 units
    // and the call updates `len` to the count it wrote.
    let queried = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
    };
    // SAFETY: `process` is an owned handle this function created and is done with.
    if let Err(error) = unsafe { CloseHandle(process) } {
        log::debug!("closing a process handle failed: {error}");
    }
    queried
        .inspect_err(|error| log::debug!("process {process_id} hid its image path: {error}"))
        .ok()?;
    Some(PathBuf::from(OsString::from_wide(&buffer[..len as usize])))
}

/// Linux: every `/proc/*/cmdline` that reads. No filter on the entry's name: past the
/// process ids only `self` and `thread-self` have one, and they are this process, never a
/// game.
#[cfg(target_os = "linux")]
fn cmdlines() -> Vec<Vec<u8>> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut cmdlines = Vec::new();
    for entry in entries.flatten() {
        if let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) {
            cmdlines.push(cmdline);
        }
    }
    cmdlines
}

/// Linux: the games among `cmdlines()`; each buffer's first argument under Wine or
/// Proton is the exe's Windows path.
#[cfg(target_os = "linux")]
fn probe() -> Vec<RunningPes> {
    cmdlines()
        .iter()
        .filter_map(|cmdline| exe_of_cmdline(cmdline))
        .map(|(version, exe)| RunningPes {
            version,
            exe: Some(exe),
        })
        .collect()
}

/// Every other platform: no probe exists.
#[cfg(not(any(windows, target_os = "linux")))]
fn probe() -> Vec<RunningPes> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_of_exe_matches_each_shipped_name_in_any_case() {
        let expected = [
            ("PES2015.exe", PesVersion::Pes15),
            ("PES2016.exe", PesVersion::Pes16),
            ("PES2017.exe", PesVersion::Pes17),
            ("PES2018.exe", PesVersion::Pes18),
            ("PES2019.exe", PesVersion::Pes19),
            ("PES2020.exe", PesVersion::Pes20),
            ("PES2021.exe", PesVersion::Pes21),
            ("pes2018.EXE", PesVersion::Pes18),
            ("Pes2021.Exe", PesVersion::Pes21),
        ];
        for (name, version) in expected {
            assert_eq!(version_of_exe(name), Some(version), "{name}");
        }
    }

    #[test]
    fn version_of_exe_matches_nothing_else() {
        for name in [
            "PES2014.exe",
            "PES2021.exe.bak",
            "pes2021",
            "xPES2021.exe",
            "pes21.exe",
        ] {
            assert_eq!(version_of_exe(name), None, "{name}");
        }
    }

    #[test]
    fn exe_of_cmdline_reads_the_first_argument_last_segment() {
        assert_eq!(
            exe_of_cmdline(b"Z:\\games\\PES2021\\PES2021.exe\0-arg\0"),
            Some((
                PesVersion::Pes21,
                PathBuf::from("Z:\\games\\PES2021\\PES2021.exe"),
            )),
        );
        assert_eq!(
            exe_of_cmdline(b"/home/x/PES2017.exe\0"),
            Some((PesVersion::Pes17, PathBuf::from("/home/x/PES2017.exe"))),
        );
    }

    #[test]
    fn exe_of_cmdline_skips_empty_and_non_game_arguments() {
        assert_eq!(exe_of_cmdline(b""), None);
        assert_eq!(exe_of_cmdline(b"wine\0"), None);
    }

    #[cfg(windows)]
    #[test]
    fn processes_lists_this_test_process() {
        let pid = std::process::id();
        let name = std::env::current_exe()
            .expect("the test binary has a path")
            .file_name()
            .expect("the path has a file name")
            .to_string_lossy()
            .into_owned();
        assert!(
            processes().contains(&(pid, name.clone())),
            "({pid}, {name}) not in the snapshot"
        );
    }

    #[cfg(windows)]
    #[test]
    fn image_path_is_this_test_binary() {
        let expected = std::env::current_exe().expect("the test binary has a path");
        assert_eq!(image_path(std::process::id()), Some(expected));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cmdlines_hold_this_test_process() {
        let own = std::fs::read("/proc/self/cmdline").expect("/proc/self is readable");
        assert!(cmdlines().contains(&own), "own cmdline not in the list");
    }

    #[test]
    fn running_pes_lists_only_supported_versions() {
        for game in running_pes() {
            assert!(PesVersion::ALL.contains(&game.version), "{game:?}");
        }
    }

    /// Kills the fake game and removes its folder on drop, so a failed
    /// assertion leaves neither behind.
    #[cfg(any(windows, target_os = "linux"))]
    struct FakeGame {
        child: std::process::Child,
        folder: PathBuf,
    }

    #[cfg(any(windows, target_os = "linux"))]
    impl Drop for FakeGame {
        fn drop(&mut self) {
            // Cleanup only: the test already killed and reaped the child, so these
            // usually fail on a dead process, and a leftover folder fails no assertion.
            drop(self.child.kill());
            drop(self.child.wait());
            drop(std::fs::remove_dir_all(&self.folder));
        }
    }

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn running_pes_lists_a_running_game_with_its_exe() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        // The probe's one job is seeing a running game (`spec/core/architecture.md` "Crate
        // structure", the running-PES probe), and every other test runs with none open. The
        // fake game is PES 2020 because the tools' tests use that version
        // least: while it lives, any other test probing at the same moment (a
        // parallel mutation job, the Save editor's or Team compiler's own
        // tests) sees a running PES 2020 and a PES 2020 write refuses, so the
        // window is kept to the one poll that lists it.
        let folder =
            std::env::temp_dir().join(format!("studio_core_running_pes_{}", std::process::id()));
        if folder.exists() {
            std::fs::remove_dir_all(&folder).unwrap();
        }
        std::fs::create_dir_all(&folder).unwrap();
        let exe = folder.join("PES2020.exe");

        // A small system program that runs until killed, copied under the name
        // the game ships with so the probe's by-name match sees it.
        #[cfg(windows)]
        let (sleeper, arguments) = (
            PathBuf::from(r"C:\Windows\System32\PING.EXE"),
            ["-n", "30", "127.0.0.1"],
        );
        #[cfg(target_os = "linux")]
        let (sleeper, arguments) = (
            ["/bin/sleep", "/usr/bin/sleep"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| path.is_file())
                .expect("sleep lives in /bin or /usr/bin"),
            ["30"],
        );
        std::fs::copy(&sleeper, &exe).expect("the sleeper program copies");

        let mut fake = FakeGame {
            child: Command::new(&exe)
                .args(arguments)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("the fake game starts"),
            folder,
        };

        // The system names a process's file by its real path, which the temp
        // folder's name need not be (a short name on a CI runner, a link in the
        // gates): both sides are compared resolved.
        let real_exe = exe.canonicalize().expect("the fake game's file resolves");
        let lists_it = |games: &[RunningPes]| {
            games.iter().any(|game| {
                game.version == PesVersion::Pes20
                    && game
                        .exe
                        .as_ref()
                        .and_then(|path| path.canonicalize().ok())
                        .as_ref()
                        == Some(&real_exe)
            })
        };

        // The probe can take a moment to see a new process: poll for up to
        // two minutes, and kill the fake game as soon as one poll lists it.
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut games;
        loop {
            games = running_pes();
            if lists_it(&games) || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        drop(fake.child.kill());
        drop(fake.child.wait());

        assert!(
            lists_it(&games),
            "PES 2020 at {real_exe:?} not in {games:?}"
        );
    }
}
