use std::sync::Mutex;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tauri::{command, State};

// 1. 定义返回给前端的数据结构
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    // CPU
    cpu_brand: String,
    cpu_usage: f32, // 全局使用率
    cpu_cores: usize, // 物理核心
    cpu_logical_cores: usize, // 逻辑核心

    // 内存 字节
    total_memory: u64,
    used_memory: u64,
    total_swap: u64,
    used_swap: u64,

    // 系统
    os_name: String,
    os_version: String,
    host_name: String,
    kernel_version: String,
    uptime: u64,
}

// 2. 定义全局状态
pub struct SystemState {
    pub sys: Mutex<System>,
}

impl SystemState {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        // 预热一次，保证第一次获取 CPU 不为 0
        sys.refresh_cpu_all();
        sys.refresh_memory();

        Self {
            sys: Mutex::new(sys),
        }
    }
}

// 3. 命令实现
#[command]
pub fn get_system_info(state: State<SystemState>) -> SystemInfo {
    let mut sys = state.sys.lock().unwrap();

    // 刷新数据
    sys.refresh_cpu_all();
    sys.refresh_memory();

    // 收集 CPU 信息
    let cpus = sys.cpus();
    let cpu_brand = cpus.first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let cpu_usage = sys.global_cpu_usage();

    // 收集系统静态信息
    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "".to_string());
    let host_name = System::host_name().unwrap_or_else(|| "Localhost".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "".to_string());

    let physical_cores = System::physical_core_count().unwrap_or(cpus.len());

    SystemInfo {
        cpu_brand,
        cpu_usage,
        cpu_cores: physical_cores,
        cpu_logical_cores: cpus.len(),

        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        total_swap: sys.total_swap(),
        used_swap: sys.used_swap(),

        os_name,
        os_version,
        host_name,
        kernel_version,
        uptime: System::uptime(),
    }
}