use std::env::VarError;

pub fn get_env_var(key: &str) -> Result<String, VarError> {
    std::env::var(key)
}

pub fn get_env_var_or_panic(key: &str) -> String {
    get_env_var(key).unwrap_or_else(|e| panic!("Failed to get env var {}: {}", key, e))
}

pub fn get_env_var_or_default(key: &str, default: &str) -> String {
    get_env_var(key).unwrap_or(default.to_string())
}

pub fn get_env_var_optional(key: &str) -> Result<Option<String>, VarError> {
    match get_env_var(key) {
        // if value is empty string, return None
        Ok(s) if s.is_empty() => Ok(None),
        Ok(s) => Ok(Some(s)),
        Err(VarError::NotPresent) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_env_car_optional_or_panic(key: &str) -> Option<String> {
    get_env_var_optional(key).unwrap_or_else(|e| panic!("Failed to get env var {}: {}", key, e))
}

use std::process;

use sysinfo::{PidExt, ProcessExt, System, SystemExt};

pub fn printtttttt(state: &str) {
    // Initialize the system information collector
    let mut sys = System::new_all();
    sys.refresh_all();

    // Get current process ID
    let pid = process::id();

    // Find our process in the system processes
    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
        // Memory in GB (converting from bytes)
        let memory_gb = process.memory() as f64 / 1024.0 / 1024.0 / 1024.0;

        // CPU usage as percentage
        let cpu_usage = process.cpu_usage();

        println!("{} Memory Usage: {:.2} GB", state, memory_gb);
        println!("{} CPU Usage: {:.1}%", state, cpu_usage);
    } else {
        println!("Could not find process information");
    }
}

use std::alloc::{GlobalAlloc, Layout};
use std::mem;

use sysinfo::System as SysSystem;

pub fn print_process_stats(state: &str) {
    // Get process-level memory stats
    let mut sys = SysSystem::new_all();
    sys.refresh_all();

    let pid = process::id();
    if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
        let memory_gb = process.memory() as f64 / 1024.0 / 1024.0 / 1024.0;

        // CPU usage as percentage
        let cpu_usage = process.cpu_usage();

        let virtual_memory = process.virtual_memory();
        let physical_memory = process.memory();

        // Calculate fragmentation ratio
        // Higher ratio indicates more fragmentation
        let fragmentation_ratio =
            if virtual_memory > 0 { (virtual_memory - physical_memory) as f64 / virtual_memory as f64 } else { 0.0 };

        println!("Memory Statistics: {}", state);
        println!("{} Memory Usage: {:.2} GB", state, memory_gb);
        println!("CPU Usage: {:.1}%", cpu_usage);
        // println!("Virtual Memory: {} bytes", virtual_memory);
        println!("Physical Memory: {} bytes", physical_memory);
        // println!("Fragmentation Ratio: {:.2}%", fragmentation_ratio * 100.0);
    }
}
