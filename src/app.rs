use std::time::Instant;

use crate::process::{self, ProcessState, ProcessTracker};
use crate::SystemOverview;

/// Represents the active column used for sorting the process table.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SortColumn {
    Cpu,
    Memory,
    Pid,
    Name,
}

impl SortColumn {
    pub fn next(&self) -> Self {
        match *self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Pid,
            Self::Pid => Self::Name,
            Self::Name => Self::Cpu,
        }
    }
}

/// A formatted, display-ready snapshot of a single process for table rendering.
#[derive(Debug, PartialEq, Clone)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub cpu_percent: f64,
    pub rss_kb: u64,
    pub threads: u64,
    pub cmdline: String,
}

/// Main state for the TUI application.
pub struct App {
    pub should_quit: bool,
    pub sort_by: SortColumn,
    pub selected_index: usize,
    pub tracker: ProcessTracker,
    pub overview: Option<SystemOverview>,
    pub processes: Vec<ProcessRow>,
}

impl App {
    /// Creates a new App with default settings.
    pub fn new() -> Self {
        Self {
            should_quit: false,
            sort_by: SortColumn::Cpu,
            selected_index: 0,
            tracker: ProcessTracker::new(),
            overview: None,
            processes: Vec::new(),
        }
    }

    /// Sets should_quit to true so the event loop terminates cleanly.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_row(&mut self) {
        if !self.processes.is_empty() && self.selected_index < self.processes.len() - 1 {
            self.selected_index += 1
        }
    }

    pub fn previous_row(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Sorts `self.processes` in-place according to the active `SortColumn`.
    pub fn sort_processes(&mut self) {
        match self.sort_by {
            SortColumn::Cpu => {
                self.processes.sort_by(|a, b| {
                    b.cpu_percent
                        .partial_cmp(&a.cpu_percent)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortColumn::Memory => {
                self.processes.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
            }
            SortColumn::Pid => {
                self.processes.sort_by_key(|p| p.pid);
            }
            SortColumn::Name => {
                self.processes
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = self.sort_by.next();
        self.sort_processes();
    }

    pub fn on_tick(&mut self) {
        if let Ok(overview) = crate::collect_system_overview() {
            self.overview = Some(overview);
        }
        if let Ok(raw_processes) = process::collect_all_processes() {
            let now = Instant::now();
            self.processes.clear();
            let mut active_pids = Vec::with_capacity(raw_processes.len());

            for info in raw_processes {
                active_pids.push(info.stat.pid);
                let cpu =
                    self.tracker
                        .calculate_cpu(info.stat.pid, info.stat.total_cpu_ticks(), now);
                let rss_kb = info.stat.rss_kb();
                self.processes.push(ProcessRow {
                    pid: info.stat.pid,
                    name: info.stat.name,
                    state: info.stat.state,
                    cpu_percent: cpu,
                    rss_kb,
                    threads: info.stat.threads,
                    cmdline: info.cmdline,
                });
            }
            self.tracker.cleanup_vanished(&active_pids);
        }
        self.sort_processes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_column_cycle() {
        let col = SortColumn::Cpu;
        assert_eq!(col.next(), SortColumn::Memory);
        assert_eq!(col.next().next(), SortColumn::Pid);
        assert_eq!(col.next().next().next(), SortColumn::Name);
        assert_eq!(col.next().next().next().next(), SortColumn::Cpu);
    }

    #[test]
    fn test_app_navigation_clamping() {
        let mut app = App::new();
        // Add 2 mock rows
        app.processes.push(ProcessRow {
            pid: 1,
            name: "init".to_string(),
            state: ProcessState::Running,
            cpu_percent: 0.0,
            rss_kb: 1000,
            threads: 1,
            cmdline: "/sbin/init".to_string(),
        });
        app.processes.push(ProcessRow {
            pid: 2,
            name: "kthreadd".to_string(),
            state: ProcessState::Sleeping,
            cpu_percent: 0.0,
            rss_kb: 0,
            threads: 1,
            cmdline: "[kthreadd]".to_string(),
        });

        assert_eq!(app.selected_index, 0);

        // Move down once -> index 1
        app.next_row();
        assert_eq!(app.selected_index, 1);

        // Move down again -> should clamp to 1 (cannot go past last row)
        app.next_row();
        assert_eq!(app.selected_index, 1);

        // Move up -> index 0
        app.previous_row();
        assert_eq!(app.selected_index, 0);

        // Move up again -> should clamp to 0
        app.previous_row();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn test_sort_processes_and_cycle() {
        let mut app = App::new();
        app.processes = vec![
            ProcessRow {
                pid: 10,
                name: "bash".to_string(),
                state: ProcessState::Sleeping,
                cpu_percent: 5.0,
                rss_kb: 2000,
                threads: 1,
                cmdline: "bash".to_string(),
            },
            ProcessRow {
                pid: 2,
                name: "cargo".to_string(),
                state: ProcessState::Running,
                cpu_percent: 25.0,
                rss_kb: 1000,
                threads: 4,
                cmdline: "cargo".to_string(),
            },
            ProcessRow {
                pid: 5,
                name: "Alacritty".to_string(),
                state: ProcessState::Sleeping,
                cpu_percent: 15.0,
                rss_kb: 5000,
                threads: 8,
                cmdline: "alacritty".to_string(),
            },
        ];

        // Default sort is Cpu (descending)
        app.sort_processes();
        assert_eq!(app.processes[0].pid, 2); // 25.0%
        assert_eq!(app.processes[1].pid, 5); // 15.0%
        assert_eq!(app.processes[2].pid, 10); // 5.0%

        // Cycle to Memory (descending)
        app.cycle_sort();
        assert_eq!(app.sort_by, SortColumn::Memory);
        assert_eq!(app.processes[0].pid, 5); // 5000 KB
        assert_eq!(app.processes[1].pid, 10); // 2000 KB
        assert_eq!(app.processes[2].pid, 2); // 1000 KB

        // Cycle to Pid (ascending)
        app.cycle_sort();
        assert_eq!(app.sort_by, SortColumn::Pid);
        assert_eq!(app.processes[0].pid, 2);
        assert_eq!(app.processes[1].pid, 5);
        assert_eq!(app.processes[2].pid, 10);

        // Cycle to Name (case-insensitive ascending)
        app.cycle_sort();
        assert_eq!(app.sort_by, SortColumn::Name);
        assert_eq!(app.processes[0].name, "Alacritty");
        assert_eq!(app.processes[1].name, "bash");
        assert_eq!(app.processes[2].name, "cargo");
    }
}
