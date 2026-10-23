use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind,
    System,
};

/// Description of the machine, written at the top of a performance report.
pub struct MachineInfo {
    pub host: String,
    pub os: String,
    pub cpu: String,
    pub cores: usize,
    pub total_memory_bytes: u64,
}

impl MachineInfo {
    pub fn detect() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        Self {
            host: System::host_name().unwrap_or_else(|| "unknown".into()),
            os: System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
            cpu: system
                .cpus()
                .first()
                .map_or_else(|| "unknown".into(), |cpu| cpu.brand().trim().to_owned()),
            cores: system.cpus().len(),
            total_memory_bytes: system.total_memory(),
        }
    }
}

/// Samples the resident memory of this process and keeps the peak.
pub struct MemorySampler {
    system: System,
    pid: Option<Pid>,
    pub peak_bytes: u64,
}

impl MemorySampler {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            pid: sysinfo::get_current_pid().ok(),
            peak_bytes: 0,
        }
    }

    pub fn sample(&mut self) {
        let Some(pid) = self.pid else {
            return;
        };
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing().with_memory(),
        );
        if let Some(process) = self.system.process(pid) {
            self.peak_bytes = self.peak_bytes.max(process.memory());
        }
    }
}
