use std::fs;

pub mod app;
pub mod process;
pub mod ui;

// ==========================================
// Step 1: System Uptime
// ==========================================

pub fn parse_uptime(content: &str) -> Result<f64, String> {
    let mut parts = content.split_whitespace();
    let first_part = parts.next().ok_or("No first part".to_string())?;
    let seconds = first_part
        .parse::<f64>()
        .map_err(|_| "Invalid number".to_string())?;
    Ok(seconds)
}

pub fn format_uptime(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let sec = total_secs % 60;
    format!("{}d {}h {}m {}s", days, hours, minutes, sec)
}

pub fn read_uptime() -> Result<f64, String> {
    let reads_content = fs::read_to_string("/proc/uptime").map_err(|err| err.to_string())?;
    let sec = parse_uptime(&reads_content).map_err(|err| err.to_string())?;
    Ok(sec)
}

// ==========================================
// Step 2: Load Average
// ==========================================

#[derive(Debug, PartialEq)]
pub struct LoadAvg {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

pub fn parse_loadavg(content: &str) -> Result<LoadAvg, String> {
    let mut parts = content.split_whitespace();
    let first_part = parts.next().ok_or("No first part".to_string())?;
    let second_part = parts.next().ok_or("No second part".to_string())?;
    let third_part = parts.next().ok_or("No third part".to_string())?;

    Ok(LoadAvg {
        one: first_part
            .parse::<f64>()
            .map_err(|_| "Invalid number".to_string())?,
        five: second_part
            .parse::<f64>()
            .map_err(|_| "Invalid number".to_string())?,
        fifteen: third_part
            .parse::<f64>()
            .map_err(|_| "Invalid number".to_string())?,
    })
}

pub fn read_loadavg() -> Result<LoadAvg, String> {
    let content = fs::read_to_string("/proc/loadavg").map_err(|err| err.to_string())?;
    let load = parse_loadavg(&content).map_err(|err| err.to_string())?;
    Ok(load)
}

// ==========================================
// Step 3: Memory Info
// ==========================================

#[derive(Debug, PartialEq)]
pub struct MemInfo {
    pub total_kb: u64,
    pub available_kb: u64,
}

impl MemInfo {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }

    pub fn used_percent(&self) -> f64 {
        if self.total_kb == 0 {
            0.0
        } else {
            (self.used_kb() as f64 / self.total_kb as f64) * 100.0
        }
    }
}

pub fn format_kb(kb: u64) -> String {
    let mb = kb as f64 / 1024.0;
    if mb >= 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

pub fn parse_meminfo(content: &str) -> Result<MemInfo, String> {
    let mut total = None;
    let mut available = None;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            let mut parts = line.split_whitespace();
            let second_part = parts.nth(1).ok_or("No second part".to_string())?;
            total = Some(
                second_part
                    .parse::<u64>()
                    .map_err(|_| "Invalid number".to_string())?,
            );
        } else if line.starts_with("MemAvailable:") {
            let mut parts = line.split_whitespace();
            let second_part = parts.nth(1).ok_or("No second part".to_string())?;
            available = Some(
                second_part
                    .parse::<u64>()
                    .map_err(|_| "Invalid number".to_string())?,
            );
        }
    }
    match (total, available) {
        (Some(total_kb), Some(available_kb)) => Ok(MemInfo {
            total_kb,
            available_kb,
        }),
        _ => Err("Missing MemTotal or MemAvailable in meminfo".to_string()),
    }
}

pub fn read_meminfo() -> Result<MemInfo, String> {
    let content = fs::read_to_string("/proc/meminfo").map_err(|err| err.to_string())?;
    let mem = parse_meminfo(&content).map_err(|err| err.to_string())?;
    Ok(mem)
}

#[derive(Debug, PartialEq)]
pub struct SystemOverview {
    pub uptime_seconds: f64,
    pub load: LoadAvg,
    pub memory: MemInfo,
}

impl SystemOverview {
    pub fn format_banner(&self) -> String {
        let uptime_str = format_uptime(self.uptime_seconds);
        let mem_used = format_kb(self.memory.used_kb());
        let mem_total = format_kb(self.memory.total_kb);
        let mem_percent = self.memory.used_percent();

        format!(
            "┌────────────────────────────────────────────────────────┐\n\
            │ System Overview                                        │\n\
            │                                                        │\n\
            │ Uptime: {}                                              │\n\
            │                                                        │\n\
            │ Load Average:                                          │\n\
            │ 1m: {:.2}                                              │\n\
            │ 5m: {:.2}                                              │\n\
            │ 15m: {:.2}                                             │\n\
            │                                                        │\n\
            │ Memory:                                                │\n\
            │ Used: {}                                              │\n\
            │ Total: {}                                             │\n\
            │ Used %: {:.2}                                          │\n\
            │                                                        │\n\
            └────────────────────────────────────────────────────────┘",
            uptime_str,
            self.load.one,
            self.load.five,
            self.load.fifteen,
            mem_used,
            mem_total,
            mem_percent
        )
    }
}

pub fn collect_system_overview() -> Result<SystemOverview, String> {
    let uptime = read_uptime()?;
    let load = read_loadavg()?;
    let mem = read_meminfo()?;
    Ok(SystemOverview {
        uptime_seconds: uptime,
        load,
        memory: mem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_uptime() {
        let sample = "13294.34 25139.13\n";
        let result = parse_uptime(sample);
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!((val - 13294.34).abs() < 1e-4);
    }

    #[test]
    fn test_parse_empty_uptime() {
        let sample = "";
        let result = parse_uptime(sample);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_uptime() {
        let sample = "not_a_number 123.45";
        let result = parse_uptime(sample);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(90061.0), "1d 1h 1m 1s");
        assert_eq!(format_uptime(65.0), "0d 0h 1m 5s");
    }

    #[test]
    fn test_parse_valid_loadavg() {
        let sample = "0.67 0.27 0.17 4/785 7655\n";
        let result = parse_loadavg(sample);
        assert!(result.is_ok());
        let load = result.unwrap();
        assert!((load.one - 0.67).abs() < 1e-4);
        assert!((load.five - 0.27).abs() < 1e-4);
        assert!((load.fifteen - 0.17).abs() < 1e-4);
    }

    #[test]
    fn test_parse_incomplete_loadavg() {
        let sample = "0.67 0.27\n";
        let result = parse_loadavg(sample);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_loadavg() {
        let sample = "0.67 bad_number 0.17\n";
        let result = parse_loadavg(sample);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_valid_meminfo() {
        let sample = "\
MemTotal:        5489220 kB
MemFree:          298656 kB
MemAvailable:    2212900 kB
Buffers:           22644 kB
Cached:          2081312 kB
";
        let result = parse_meminfo(sample);
        assert!(result.is_ok());
        let mem = result.unwrap();
        assert_eq!(mem.total_kb, 5489220);
        assert_eq!(mem.available_kb, 2212900);
        assert_eq!(mem.used_kb(), 5489220 - 2212900);
    }

    #[test]
    fn test_parse_missing_field_meminfo() {
        let sample = "MemTotal: 5489220 kB\nMemFree: 298656 kB\n";
        let result = parse_meminfo(sample);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_kb() {
        assert_eq!(format_kb(1024), "1.0 MB");
        assert_eq!(format_kb(2097152), "2.00 GB");
    }

    #[test]
    fn test_system_overview_ownership_and_banner() {
        let overview = SystemOverview {
            uptime_seconds: 3661.0,
            load: LoadAvg {
                one: 0.50,
                five: 0.75,
                fifteen: 1.00,
            },
            memory: MemInfo {
                total_kb: 4194304,
                available_kb: 2097152,
            },
        };

        let banner = overview.format_banner();
        // The banner should contain key information
        assert!(banner.contains("1h 1m 1s"));
        assert!(banner.contains("0.50"));
        assert!(banner.contains("2.00 GB"));
    }
}
