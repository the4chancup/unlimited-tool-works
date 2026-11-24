//! The Win32 calls behind the runner's memory cap: the completion port the job's limit
//! messages arrive on, the Job Object that carries the cap, and the drain that reads them.

use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOBOBJECT_ASSOCIATE_COMPLETION_PORT,
    JOBOBJECT_BASIC_LIMIT_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectAssociateCompletionPortInformation, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows::Win32::System::SystemServices::JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::core::PCWSTR;

/// A fresh I/O completion port for the job's limit messages.
pub(crate) fn create_port() -> Result<HANDLE, String> {
    // SAFETY: `INVALID_HANDLE_VALUE` with no existing port creates a new port; the
    // returned handle is ours and is kept open for the process's life.
    unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1) }
        .map_err(|error| failed("CreateIoCompletionPort", error))
}

/// A Job Object capping each member process's commit at `cap` bytes and posting its
/// messages to `port`. `KILL_ON_JOB_CLOSE` kills every member the moment the handle
/// closes, so the caller keeps it open for the runner's whole life: closing it would
/// kill the runner itself, and a timeout kill of the runner relies on the child dying
/// with it instead of being orphaned.
pub(crate) fn create_job(cap: u64, port: HANDLE) -> Result<HANDLE, String> {
    // SAFETY: no attributes and no name create an anonymous job; the returned handle is
    // ours and outlives every call below.
    let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
        .map_err(|error| failed("CreateJobObjectW", error))?;
    let info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
            LimitFlags: JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            ..Default::default()
        },
        ProcessMemoryLimit: cap.try_into().unwrap_or(usize::MAX),
        ..Default::default()
    };
    // SAFETY: `job` is the live handle from above and `info` is a correctly sized local
    // of the struct the `JobObjectExtendedLimitInformation` class expects.
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const std::ffi::c_void,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    }
    .map_err(|error| failed("SetInformationJobObject", error))?;
    let assoc = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
        CompletionKey: std::ptr::null_mut(),
        CompletionPort: port,
    };
    // SAFETY: same job and call; `assoc` is the correctly sized local the
    // `JobObjectAssociateCompletionPortInformation` class expects.
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectAssociateCompletionPortInformation,
            &assoc as *const JOBOBJECT_ASSOCIATE_COMPLETION_PORT as *const std::ffi::c_void,
            size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
        )
    }
    .map_err(|error| failed("SetInformationJobObject", error))?;
    Ok(job)
}

/// Adds this process to `job`, so a child spawned afterwards inherits the membership.
/// The runner is tiny and nowhere near the cap.
pub(crate) fn assign_self_to_job(job: HANDLE) -> Result<(), String> {
    // SAFETY: `GetCurrentProcess` returns this process's pseudo-handle, always valid for
    // `AssignProcessToJobObject`; `job` is the live job handle the caller keeps open.
    unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) }
        .map_err(|error| failed("AssignProcessToJobObject", error))
}

/// Whether a job member hit its per-process memory cap: drains `port` without blocking
/// (a 0 ms timeout) until it is empty. Each job message arrives as
/// `lpNumberOfBytesTransferred` = a `JOB_OBJECT_MSG_*` value, and the port is empty when
/// the call fails with no packet.
pub(crate) fn hit_memory_limit(port: HANDLE) -> bool {
    let mut hit = false;
    loop {
        let mut bytes = 0u32;
        let mut key = 0usize;
        let mut overlapped = std::ptr::null_mut();
        // SAFETY: `bytes`, `key` and `overlapped` are correctly typed locals for the
        // call's out parameters, and `port` is the live port handle the caller holds.
        let ok =
            unsafe { GetQueuedCompletionStatus(port, &mut bytes, &mut key, &mut overlapped, 0) };
        if ok.is_err() && overlapped.is_null() {
            return hit;
        }
        if ok.is_ok() && bytes == JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT {
            hit = true;
        }
    }
}

/// `call`'s failure as the runner's one-line report.
fn failed(call: &str, error: windows::core::Error) -> String {
    format!("test_runner: {call} failed: {error}")
}
