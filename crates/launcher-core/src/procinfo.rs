//! Recognizing a project's processes again after the Launcher itself stopped unexpectedly.
//! A pid alone can be reused by an unrelated program, so every run also records when its
//! process started and when the computer booted; a process only counts as ours if those match.

use crate::store::RunRecord;
use std::time::Duration;

/// Start times are compared with this tolerance: Linux derives them from the boot time, which
/// clock adjustments can nudge.
const SLACK_MS: i64 = 1000;
/// Boot times this far apart mean the computer restarted since the run began.
const REBOOT_MS: i64 = 60_000;

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Whether the run's process tree is still going and is really the one we started.
pub fn still_running(r: &RunRecord) -> bool {
    let (Some(pid), Some(start)) = (r.pid, r.proc_start_ms) else {
        return false;
    };
    if let (Some(then), Some(now)) = (r.boot_ms, boot_time_ms())
        && (then - now).abs() > REBOOT_MS
    {
        return false;
    }
    let same_start = |t: i64| (t - start).abs() <= SLACK_MS;
    #[cfg(unix)]
    if let Some(pgid) = r.pgid {
        // The group outlives its leader when the leader exits first (`npm` → `node`). The
        // leader, if still there, must be the process we started, and everything else in the
        // group must have started after it; a reused id fails one of these.
        let members = group_members(pgid);
        return !members.is_empty()
            && members.iter().all(|&m| match start_time_ms(m) {
                Some(t) if m == pgid => same_start(t),
                Some(t) => t >= start - SLACK_MS,
                None => true, // exited while we looked
            });
    }
    // Windows: projects run in a job that dies with the Launcher; only the program itself
    // needs checking.
    start_time_ms(pid).is_some_and(same_start)
}

/// Stops a run's tree that the Launcher didn't start this session: politely first, then by
/// force after `grace`. Checks again right before each signal that it's still ours.
pub async fn stop(r: &RunRecord, grace: Duration) {
    #[cfg(unix)]
    {
        let target = match (r.pgid, r.pid) {
            (Some(g), _) => -(g as i32),
            (None, Some(p)) => p as i32,
            _ => return,
        };
        if !still_running(r) {
            return;
        }
        // SAFETY: plain signal delivery to a process group recorded (and re-verified) as ours.
        unsafe { libc::kill(target, libc::SIGTERM) };
        let deadline = tokio::time::Instant::now() + grace;
        while tokio::time::Instant::now() < deadline {
            if !still_running(r) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if still_running(r) {
            unsafe { libc::kill(target, libc::SIGKILL) };
        }
    }
    #[cfg(windows)]
    {
        let _ = grace;
        let Some(pid) = r.pid else { return };
        if !still_running(r) {
            return;
        }
        let taskkill = std::env::var_os("SYSTEMROOT")
            .map(|root| {
                std::path::Path::new(&root)
                    .join("System32")
                    .join("taskkill.exe")
            })
            .unwrap_or_else(|| "taskkill".into());
        let _ = tokio::process::Command::new(taskkill)
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .output()
            .await;
    }
}

/// When a process started, in ms since the epoch; `None` if it isn't running.
#[cfg(target_os = "macos")]
pub fn start_time_ms(pid: u32) -> Option<i64> {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: the buffer is a zeroed proc_bsdinfo of exactly the size we pass.
    let n = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    // A zombie keeps its entry until reaped; it isn't running any more.
    (n == size && info.pbi_status != libc::SZOMB)
        .then(|| info.pbi_start_tvsec as i64 * 1000 + info.pbi_start_tvusec as i64 / 1000)
}

#[cfg(target_os = "macos")]
pub fn boot_time_ms() -> Option<i64> {
    let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::timeval>();
    // SAFETY: kern.boottime is a struct timeval; the length is passed in and checked.
    let rc = unsafe {
        libc::sysctlbyname(
            c"kern.boottime".as_ptr(),
            (&mut tv as *mut libc::timeval).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (rc == 0).then(|| tv.tv_sec * 1000 + tv.tv_usec as i64 / 1000)
}

#[cfg(target_os = "macos")]
fn group_members(pgid: u32) -> Vec<u32> {
    let mut pids = vec![0i32; 1024];
    let bytes = (pids.len() * std::mem::size_of::<i32>()) as i32;
    // SAFETY: the buffer holds `bytes` bytes; the call returns how many pids it wrote.
    let n = unsafe { libc::proc_listpgrppids(pgid as i32, pids.as_mut_ptr().cast(), bytes) };
    pids.truncate(n.max(0) as usize);
    pids.into_iter()
        .filter(|&p| p > 0)
        .map(|p| p as u32)
        .filter(|&p| start_time_ms(p).is_some())
        .collect()
}

/// Fields of `/proc/<pid>/stat` after the command name: (state, process group, start ticks).
#[cfg(target_os = "linux")]
fn proc_stat(pid: u32) -> Option<(char, u32, u64)> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name may contain spaces and parentheses; the fields follow the last ')'.
    let rest = &text[text.rfind(')')? + 2..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let state = fields.first()?.chars().next()?;
    Some((
        state,
        fields.get(2)?.parse().ok()?,
        fields.get(19)?.parse().ok()?,
    ))
}

#[cfg(target_os = "linux")]
pub fn start_time_ms(pid: u32) -> Option<i64> {
    let (state, _, ticks) = proc_stat(pid)?;
    if state == 'Z' || state == 'X' {
        return None;
    }
    // SAFETY: sysconf has no preconditions.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as i64;
    Some(boot_time_ms()? + ticks as i64 * 1000 / hz)
}

#[cfg(target_os = "linux")]
pub fn boot_time_ms() -> Option<i64> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let line = stat.lines().find(|l| l.starts_with("btime "))?;
    Some(line[6..].trim().parse::<i64>().ok()? * 1000)
}

#[cfg(target_os = "linux")]
fn group_members(pgid: u32) -> Vec<u32> {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    dir.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&pid| {
            proc_stat(pid).is_some_and(|(state, g, _)| g == pgid && state != 'Z' && state != 'X')
        })
        .collect()
}

#[cfg(windows)]
pub fn start_time_ms(pid: u32) -> Option<i64> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: the handle is checked and closed; the out-parameters are plain structs.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut code = 0u32;
        let alive = GetExitCodeProcess(handle, &mut code).is_ok() && code == STILL_ACTIVE.0 as u32;
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        let timed =
            GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user).is_ok();
        let _ = CloseHandle(handle);
        if !alive || !timed {
            return None;
        }
        // 100 ns intervals since 1601 → ms since 1970.
        let t = ((created.dwHighDateTime as u64) << 32 | created.dwLowDateTime as u64) as i64;
        Some((t - 116_444_736_000_000_000) / 10_000)
    }
}

#[cfg(windows)]
pub fn boot_time_ms() -> Option<i64> {
    // SAFETY: no preconditions.
    let up = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() };
    Some(now_ms() - up as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(pid: u32, pgid: Option<u32>, start: Option<i64>) -> RunRecord {
        RunRecord {
            id: 1,
            project_id: "p".into(),
            pid: Some(pid),
            pgid,
            proc_start_ms: start,
            boot_ms: boot_time_ms(),
            url: None,
        }
    }

    #[test]
    fn this_process_is_recognized_only_with_its_start_time() {
        let pid = std::process::id();
        let start = start_time_ms(pid).expect("our own start time");
        assert!((start - now_ms()).abs() < 24 * 3600 * 1000);
        assert!(still_running(&record(pid, None, Some(start))));
        // Same pid, different start: a reused pid, not ours.
        assert!(!still_running(&record(pid, None, Some(start - 60_000))));
        assert!(!still_running(&record(pid, None, None)));
        // A run from before the computer restarted.
        let mut old = record(pid, None, Some(start));
        old.boot_ms = old.boot_ms.map(|b| b - 3_600_000);
        assert!(!still_running(&old));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_group_is_found_and_stopped() {
        use std::os::unix::process::CommandExt;
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "sleep 30 & sleep 30"])
            .process_group(0)
            .spawn()
            .unwrap();
        let pid = child.id();
        std::thread::sleep(Duration::from_millis(200));
        let r = record(pid, Some(pid), start_time_ms(pid));
        assert!(still_running(&r));
        assert!(group_members(pid).len() >= 2, "the shell and its sleeps");
        stop(&r, Duration::from_secs(3)).await;
        let _ = child.wait();
        assert!(!still_running(&r));
        assert!(group_members(pid).is_empty());
    }
}
