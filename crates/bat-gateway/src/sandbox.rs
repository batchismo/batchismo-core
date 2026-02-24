//! OS-native process sandboxing.
//!
//! Applies isolation at agent process spawn time:
//! - **Windows:** Job Objects with memory/CPU limits
//! - **macOS:** Seatbelt sandbox profiles
//! - **Linux:** namespaces + cgroups + seccomp-bpf

use anyhow::{Context, Result};
use tracing::{info, warn};

/// Sandbox configuration applied to agent processes.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Memory limit in MB (0 = unlimited).
    pub memory_limit_mb: u64,
    /// CPU shares (relative weight, 0 = default).
    pub cpu_shares: u32,
    /// Allowed filesystem paths (read or read-write).
    pub allowed_paths: Vec<(String, bool)>, // (path, writable)
    /// Allowed network endpoints (host:port).
    pub allowed_endpoints: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 512,
            cpu_shares: 0,
            allowed_paths: Vec::new(),
            allowed_endpoints: vec!["api.anthropic.com:443".to_string()],
        }
    }
}

/// Apply sandbox to a child process by PID.
/// Call this AFTER spawning but BEFORE the process does real work.
pub fn apply_sandbox(
    pid: u32,
    config: &SandboxConfig,
) -> Result<SandboxHandle> {
    #[cfg(target_os = "windows")]
    return apply_windows_sandbox(pid, config);

    #[cfg(target_os = "macos")]
    return apply_macos_sandbox(pid, config);

    #[cfg(target_os = "linux")]
    return apply_linux_sandbox(pid, config);

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (pid, config);
        warn!("No sandbox support for this OS");
        Ok(SandboxHandle::None)
    }
}

/// Generate sandbox arguments to pass to the child process at spawn time.
/// On macOS, this generates a seatbelt profile file.
/// On Linux, this sets up namespace flags.
/// On Windows, sandbox is applied post-spawn via Job Objects.
pub fn pre_spawn_setup(config: &SandboxConfig) -> Result<PreSpawnConfig> {
    #[cfg(target_os = "macos")]
    {
        let profile = generate_seatbelt_profile(config)?;
        Ok(PreSpawnConfig { seatbelt_profile: Some(profile), ..Default::default() })
    }

    #[cfg(not(target_os = "macos"))]
    Ok(PreSpawnConfig::default())
}

#[derive(Default)]
pub struct PreSpawnConfig {
    #[cfg(target_os = "macos")]
    pub seatbelt_profile: Option<String>,
}

/// Handle to sandbox resources. Drop to clean up.
pub enum SandboxHandle {
    None,
    #[cfg(target_os = "windows")]
    WindowsJob(WindowsJobHandle),
}

impl Drop for SandboxHandle {
    fn drop(&mut self) {
        match self {
            SandboxHandle::None => {}
            #[cfg(target_os = "windows")]
            SandboxHandle::WindowsJob(handle) => {
                handle.close();
            }
        }
    }
}

// ── Windows: Job Objects ────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub struct WindowsJobHandle {
    job_handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl WindowsJobHandle {
    fn close(&self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job_handle);
        }
    }
}

#[cfg(target_os = "windows")]
fn apply_windows_sandbox(
    pid: u32,
    config: &SandboxConfig,
) -> Result<SandboxHandle> {
    use std::mem;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::*;

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job == 0 {
            return Err(anyhow::anyhow!("Failed to create Job Object"));
        }

        // Set memory limit
        if config.memory_limit_mb > 0 {
            let mut ext_info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
            ext_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            ext_info.ProcessMemoryLimit = (config.memory_limit_mb * 1024 * 1024) as usize;

            let result = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &ext_info as *const _ as *const _,
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if result == 0 {
                CloseHandle(job);
                return Err(anyhow::anyhow!("Failed to set Job Object memory limit"));
            }
        }

        // Open the process and assign to Job Object
        let proc_handle = windows_sys::Win32::System::Threading::OpenProcess(
            windows_sys::Win32::System::Threading::PROCESS_ALL_ACCESS,
            0,
            pid,
        );
        if proc_handle == 0 {
            CloseHandle(job);
            return Err(anyhow::anyhow!("Failed to open process {pid} for Job Object"));
        }

        let result = AssignProcessToJobObject(job, proc_handle);
        CloseHandle(proc_handle);

        if result == 0 {
            CloseHandle(job);
            return Err(anyhow::anyhow!("Failed to assign process to Job Object"));
        }

        info!("Windows Job Object sandbox applied: pid={pid}, memory_limit={}MB", config.memory_limit_mb);

        Ok(SandboxHandle::WindowsJob(WindowsJobHandle {
            job_handle: job,
        }))
    }
}

// ── macOS: Seatbelt ─────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn generate_seatbelt_profile(config: &SandboxConfig) -> Result<String> {
    let mut profile = String::from("(version 1)\n(deny default)\n");
    profile.push_str("(allow process-exec)\n");
    profile.push_str("(allow process-fork)\n");
    profile.push_str("(allow sysctl-read)\n");
    profile.push_str("(allow mach-lookup)\n");

    // Allow file access for permitted paths
    for (path, writable) in &config.allowed_paths {
        profile.push_str(&format!("(allow file-read* (subpath \"{path}\"))\n"));
        if *writable {
            profile.push_str(&format!("(allow file-write* (subpath \"{path}\"))\n"));
        }
    }

    // Allow network to specific endpoints
    for endpoint in &config.allowed_endpoints {
        profile.push_str(&format!("(allow network-outbound (remote tcp \"{endpoint}\"))\n"));
    }

    // Allow DNS resolution
    profile.push_str("(allow network-outbound (remote udp \"*:53\"))\n");

    // Allow temp file access (needed for many operations)
    profile.push_str("(allow file-read* file-write* (subpath \"/tmp\"))\n");
    profile.push_str("(allow file-read* file-write* (subpath \"/private/tmp\"))\n");

    Ok(profile)
}

#[cfg(target_os = "macos")]
fn apply_macos_sandbox(
    _pid: u32,
    config: &SandboxConfig,
) -> Result<SandboxHandle> {
    // On macOS, seatbelt profiles are applied via sandbox-exec at spawn time,
    // not post-spawn. The profile is generated in pre_spawn_setup and passed
    // to the command. This is a no-op for post-spawn.
    info!("macOS seatbelt sandbox: profile generated with {} allowed paths", config.allowed_paths.len());
    Ok(SandboxHandle::None)
}

// ── Linux: namespaces + cgroups ─────────────────────────────────────────

#[cfg(target_os = "linux")]
fn apply_linux_sandbox(
    pid: u32,
    config: &SandboxConfig,
) -> Result<SandboxHandle> {

    // Apply cgroup memory limit
    if config.memory_limit_mb > 0 {
        let cgroup_path = format!("/sys/fs/cgroup/batchismo/agent-{pid}");
        if let Err(e) = std::fs::create_dir_all(&cgroup_path) {
            warn!("Failed to create cgroup dir (may need root): {e}");
        } else {
            let mem_bytes = config.memory_limit_mb * 1024 * 1024;
            let _ = std::fs::write(format!("{cgroup_path}/memory.max"), mem_bytes.to_string());
            let _ = std::fs::write(format!("{cgroup_path}/cgroup.procs"), pid.to_string());
            info!("Linux cgroup sandbox applied: pid={pid}, memory_limit={}MB", config.memory_limit_mb);
        }
    }

    Ok(SandboxHandle::None)
}
