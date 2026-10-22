use std::collections::HashMap;
use std::fs;
use std::time::Instant;

/// Clock ticks per second in Linux (typically 100).
pub const CLK_TCK: f64 = 100.0;

/// Standard Linux page size in bytes (4096 bytes / 4 KB).
pub const PAGE_SIZE: u64 = 4096;

/// Represents the process state as reported by `/proc/<pid>/stat`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ProcessState {
    Running,     // 'R'
    Sleeping,    // 'S'
    DiskSleep,   // 'D' (uninterruptible disk sleep)
    Zombie,      // 'Z'
    Stopped,     // 'T'
    TracingStop, // 't'
    Idle,        // 'I'
    Unknown(char),
}

impl ProcessState {
    pub fn from_char(ch: char) -> Self {
        match ch {
            'R' => ProcessState::Running,
            'S' => ProcessState::Sleeping,
            'D' => ProcessState::DiskSleep,
            'Z' => ProcessState::Zombie,
            'T' => ProcessState::Stopped,
            't' => ProcessState::TracingStop,
            'I' => ProcessState::Idle,
            _ => ProcessState::Unknown(ch),
        }    
    }
}

/// Core snapshot of a Linux process parsed from `/proc/<pid>/stat`.
#[derive(Debug, PartialEq, Clone)]
pub struct ProcessStat {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub ppid: u32,
    pub utime: u64,
    pub stime: u64,
    pub threads: u64,
    pub rss_pages: u64,
}

impl ProcessStat {
    pub fn total_cpu_ticks(&self) -> u64 {
        return self.utime + self.stime;
    }

    pub fn rss_bytes(&self) -> u64 {
        return self.rss_pages * PAGE_SIZE as u64;
    }

    pub fn rss_kb(&self) -> u64 {
        return self.rss_bytes() / 1024;
    }
}

pub fn parse_stat(content: &str) -> Result<ProcessStat, String> {
    let start = content.find('(').ok_or("Missing '(' ")?;
    let end = content.rfind(')').ok_or("Missing ')' ")?;
    let pid = content[..start].trim().parse::<u32>().map_err(|_| "Failed to parse pid")?;
    let name = content[start + 1..end].to_string();
    let rest = &content[end + 1..];

    let fields : Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 22 {
        return Err("Not enough fields".to_string());
    }
    let state_char = fields[0].chars().next().ok_or("Empty state")?;
    let state = ProcessState::from_char(state_char);
    let ppid = fields[1].parse::<u32>().map_err(|_| "Invalid ppid")?;
    let utime = fields[11].parse::<u64>().map_err(|_| "Invalid utime")?;
    let stime = fields[12].parse::<u64>().map_err(|_| "Invalid stime")?;
    let threads = fields[17].parse::<u64>().map_err(|_| "Invalid threads")?;
    let rss_pages = fields[21].parse::<u64>().map_err(|_| "Invalid rss")?;

    Ok(ProcessStat { pid, name, state, ppid, utime, stime, threads, rss_pages })
}

pub fn read_stat(pid: u32) -> Result<ProcessStat, String> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {}", path, err))?;
    parse_stat(&content)
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProcessInfo {
    pub stat: ProcessStat,
    pub cmdline: String,
}

pub fn parse_cmdline(content: &str, fallback_name: &str) -> String {
    let arg : Vec<&str> = content.split('\0').filter(|s| !s.is_empty()).collect();
    let joined = arg.join(" ");
    if joined.is_empty() {
        format!("[{}]", fallback_name)
    } else {
        joined
    }
}

pub fn read_cmdline(pid: u32, fallback_name: &str) -> String {
    let path = format!("/proc/{}/cmdline", pid);
    match fs::read_to_string(&path) {
        Ok(content) => parse_cmdline(&content, fallback_name),
        Err(_) => format!("[{}]", fallback_name),
    }
}

pub fn collect_process_info(pid: u32) -> Result<ProcessInfo, String> {
    let stat = read_stat(pid)?;
    let cmdline = read_cmdline(pid, &stat.name);
    Ok(ProcessInfo { stat, cmdline })
}


pub fn list_pids() -> Result<Vec<u32>, String> {

    let entries = fs::read_dir("/proc").map_err(|e| format!("Failed to read /proc: {}", e))?;
    let mut pids  = Vec::new();
    for entry in entries { 
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if let Ok(pid) = name.parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }
    pids.sort();
    Ok(pids)
}

pub fn collect_all_processes() -> Result<Vec<ProcessInfo>, String> {
    let pids = list_pids()?;
    let mut processes = Vec::new();
    for pid in pids {
        if let Ok(info) = collect_process_info(pid) {
            processes.push(info);
        }
    }
    Ok(processes)
}

#[derive(Debug, Default)]
pub struct ProcessTracker {
    pub prev_samples: HashMap<u32, (u64, Instant)>,
}

impl ProcessTracker {
    /// Creates a new empty tracker.
    pub fn new() -> Self {
        Self {
            prev_samples: HashMap::new(),
        }
    }

    pub fn calculate_cpu(&mut self, pid: u32, current_ticks: u64, now: Instant) -> f64 {
        let cpu_percent = if let Some(&(prev_ticks, prev_time)) = self.prev_samples.get(&pid) {
            let delta_ticks = current_ticks.saturating_sub(prev_ticks) as f64;
            let elapsed_sec = now.duration_since(prev_time).as_secs_f64();
            if elapsed_sec <= 0.0 {
                0.0
            } else {
                ((delta_ticks / CLK_TCK) / elapsed_sec) * 100.0
            }
        } else {
            0.0
        };
        self.prev_samples.insert(pid, (current_ticks, now));
        cpu_percent
    }

    pub fn cleanup_vanished(&mut self, active_pids: &[u32]) {
        self.prev_samples.retain(|pid, _| active_pids.contains(pid));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_state_from_char() {
        assert_eq!(ProcessState::from_char('R'), ProcessState::Running);
        assert_eq!(ProcessState::from_char('S'), ProcessState::Sleeping);
        assert_eq!(ProcessState::from_char('Z'), ProcessState::Zombie);
        assert_eq!(ProcessState::from_char('X'), ProcessState::Unknown('X'));
    }

    #[test]
    fn test_parse_simple_stat() {
        let sample = "10733 (cat) R 3407 10733 10733 34819 10733 4194304 611 0 1 0 4 2 0 0 20 0 1 0 1045882 18214912 1880 0 0 0\n";
        let stat = parse_stat(sample).expect("Failed to parse valid stat");
        assert_eq!(stat.pid, 10733);
        assert_eq!(stat.name, "cat");
        assert_eq!(stat.state, ProcessState::Running);
        assert_eq!(stat.ppid, 3407);
        assert_eq!(stat.utime, 4);
        assert_eq!(stat.stime, 2);
        assert_eq!(stat.threads, 1);
        assert_eq!(stat.rss_pages, 1880);
    }

    #[test]
    fn test_parse_name_with_spaces_and_parentheses() {
        // A tricky process name: "(worker) thread (1)"
        let sample = "456 ((worker) thread (1)) S 100 456 456 0 0 0 0 0 0 0 10 5 0 0 20 0 8 0 0 1000 500 0 0 0\n";
        let stat = parse_stat(sample).expect("Failed to parse tricky stat name");
        assert_eq!(stat.pid, 456);
        assert_eq!(stat.name, "(worker) thread (1)");
        assert_eq!(stat.state, ProcessState::Sleeping);
        assert_eq!(stat.ppid, 100);
        assert_eq!(stat.utime, 10);
        assert_eq!(stat.stime, 5);
        assert_eq!(stat.threads, 8);
        assert_eq!(stat.rss_pages, 500);
    }

    #[test]
    fn test_parse_invalid_stat() {
        assert!(parse_stat("").is_err());
        assert!(parse_stat("10733 no_parentheses R").is_err());
        assert!(parse_stat("not_a_pid (test) S 1 2 3").is_err());
    }

    #[test]
    fn test_parse_cmdline_multiple_args() {
        let sample = "python3\0server.py\0--port\08080\0";
        assert_eq!(parse_cmdline(sample, "python3"), "python3 server.py --port 8080");
    }

    #[test]
    fn test_parse_cmdline_kernel_thread() {
        let sample = "";
        assert_eq!(parse_cmdline(sample, "kthreadd"), "[kthreadd]");
    }

    #[test]
    fn test_parse_cmdline_trailing_nulls() {
        let sample = "bash\0\0\0";
        assert_eq!(parse_cmdline(sample, "bash"), "bash");
    }

    #[test]
    fn test_list_pids_finds_processes() {
        let pids = list_pids().expect("Failed to list pids");
        assert!(!pids.is_empty(), "PIDs list should not be empty");
        // PID 1 (init / systemd) is always present on Linux
        assert!(pids.contains(&1), "PID 1 should be present in /proc");
    }

    #[test]
    fn test_collect_all_processes_non_empty() {
        let procs = collect_all_processes().expect("Failed to collect processes");
        assert!(!procs.is_empty(), "Processes list should not be empty");
        // Ensure at least one process has a valid name and state
        let init = procs.iter().find(|p| p.stat.pid == 1);
        assert!(init.is_some(), "Init process (PID 1) should be found");
    }

    #[test]
    fn test_process_memory_helpers() {
        let stat = ProcessStat {
            pid: 1234,
            name: "test".to_string(),
            state: ProcessState::Running,
            ppid: 1,
            utime: 50,
            stime: 25,
            threads: 4,
            rss_pages: 250, // 250 * 4096 = 1,024,000 bytes = 1000 KB
        };

        assert_eq!(stat.total_cpu_ticks(), 75);
        assert_eq!(stat.rss_bytes(), 1_024_000);
        assert_eq!(stat.rss_kb(), 1000);
    }

    #[test]
    fn test_cpu_calculation_two_samples() {
        use std::time::Duration;

        let mut tracker = ProcessTracker::new();
        let t1 = Instant::now();

        // Sample 1: first observation should always return 0.0%
        let cpu1 = tracker.calculate_cpu(100, 1000, t1);
        assert_eq!(cpu1, 0.0);

        // Sample 2: 1.0 second later, 100 ticks consumed (100 ticks/sec at CLK_TCK 100 = 100% of 1 core)
        let t2 = t1 + Duration::from_secs(1);
        let cpu2 = tracker.calculate_cpu(100, 1100, t2);
        assert!((cpu2 - 100.0).abs() < 1e-2);

        // Test cleanup of vanished PID
        tracker.cleanup_vanished(&[200]); // PID 100 is not in active list
        assert!(!tracker.prev_samples.contains_key(&100));
    }
}
