use anyhow::Result;

pub struct SystemInfo;

impl SystemInfo {
    pub fn new() -> Self {
        Self
    }
}

impl super::ToolExecutor for SystemInfo {
    fn name(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "Get system information: OS, hostname, CPU, memory usage, disk space, and environment details."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute(&self, _input: &serde_json::Value) -> Result<String> {
        let mut info = String::new();

        // OS info
        info.push_str(&format!("OS: {} {}\n", std::env::consts::OS, std::env::consts::ARCH));

        // Hostname
        if let Ok(hostname) = hostname::get() {
            info.push_str(&format!("Hostname: {}\n", hostname.to_string_lossy()));
        }

        // Current user
        if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
            info.push_str(&format!("User: {user}\n"));
        }

        // Home directory
        if let Some(home) = dirs::home_dir() {
            info.push_str(&format!("Home: {}\n", home.display()));
        }

        // Current directory
        if let Ok(cwd) = std::env::current_dir() {
            info.push_str(&format!("CWD: {}\n", cwd.display()));
        }

        // CPU count
        info.push_str(&format!("CPUs: {}\n", num_cpus::get()));

        // Memory info via command
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("wmic")
                .args(["OS", "get", "TotalVisibleMemorySize,FreePhysicalMemory", "/format:list"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if line.starts_with("TotalVisibleMemorySize=") || line.starts_with("FreePhysicalMemory=") {
                        let parts: Vec<&str> = line.split('=').collect();
                        if parts.len() == 2 {
                            if let Ok(kb) = parts[1].trim().parse::<u64>() {
                                let gb = kb as f64 / 1_048_576.0;
                                let label = if line.starts_with("Total") { "Total Memory" } else { "Free Memory" };
                                info.push_str(&format!("{label}: {gb:.1} GB\n"));
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(output) = std::process::Command::new("sh")
                .args(["-c", "free -h | head -2"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                info.push_str(&format!("Memory:\n{text}\n"));
            }
        }

        // Disk usage
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("wmic")
                .args(["logicaldisk", "where", "DriveType=3", "get", "Name,Size,FreeSpace", "/format:list"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut current_drive = String::new();
                let mut free = 0u64;
                let mut total: u64;
                for line in text.lines() {
                    if line.starts_with("Name=") {
                        current_drive = line.replace("Name=", "").trim().to_string();
                    } else if line.starts_with("FreeSpace=") {
                        free = line.replace("FreeSpace=", "").trim().parse().unwrap_or(0);
                    } else if line.starts_with("Size=") {
                        total = line.replace("Size=", "").trim().parse().unwrap_or(0);
                        if total > 0 {
                            let free_gb = free as f64 / 1_073_741_824.0;
                            let total_gb = total as f64 / 1_073_741_824.0;
                            info.push_str(&format!("Disk {current_drive}: {free_gb:.1} GB free / {total_gb:.1} GB total\n"));
                        }
                    }
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(output) = std::process::Command::new("df")
                .args(["-h", "/"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                info.push_str(&format!("Disk:\n{text}\n"));
            }
        }

        Ok(info)
    }
}
