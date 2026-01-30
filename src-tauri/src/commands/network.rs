use std::collections::HashMap;
use std::process::Command;
use tauri::command;

#[derive(serde::Serialize)]
pub struct PortInfo {
    pid: String,
    port: String,
    protocol: String,
    program: String,
}

#[command]
pub fn scan_ports() -> Result<Vec<PortInfo>, String> {
    let mut ports = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // --- 1: 获取进程列表并建立 PID -> 名字 的映射 ---
        let mut pid_map = HashMap::new();

        // 执行 tasklist /FO CSV /NH
        let tasklist_output = Command::new("tasklist")
            .args(&["/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok(); // 这里使用 ok() 忽略错误，如果获取失败就只显示 PID

        if let Some(output) = tasklist_output {
            // 注意：中文 Windows 默认编码可能是 GBK，这里直接当 UTF-8 处理
            // 大多数 exe 名字是英文，不会乱码。如果遇到中文路径可能会有乱码，但不会影响程序崩溃。
            let stdout = String::from_utf8_lossy(&output.stdout);

            for line in stdout.lines() {
                // tasklist CSV 格式示例: "svchost.exe","1234","Services","0","12,345 K"
                // 简单的解析逻辑：
                let parts: Vec<&str> = line.split("\",\"").collect();
                if parts.len() >= 2 {
                    // 去除首尾可能的引号
                    let name = parts[0].trim_matches('"');
                    let pid = parts[1].trim_matches('"');
                    pid_map.insert(pid.to_string(), name.to_string());
                }
            }
        }

        // --- 2: 执行 netstat -ano 获取端口信息 ---
        let output = Command::new("netstat")
            .args(&["-ano"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // 典型格式: TCP 0.0.0.0:80 0.0.0.0:0 LISTENING 1234
            if parts.len() >= 5 && parts[0].eq_ignore_ascii_case("TCP") {
                let local_addr = parts[1];
                let state = parts[3];
                let pid = parts[parts.len() - 1];

                if state == "LISTENING" {
                    let port = local_addr.split(':').last().unwrap_or("?").to_string();

                    // --- 查表获取进程名 ---
                    // 如果查不到，就默认显示为空字符串或者再次显示 PID
                    let program = pid_map.get(pid).cloned().unwrap_or_default();

                    ports.push(PortInfo {
                        pid: pid.to_string(),
                        port,
                        protocol: "TCP".to_string(),
                        program,
                    });
                }
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // 执行 lsof -i -P -n -sTCP:LISTEN
        let output = Command::new("lsof")
            .args(&["-iTCP", "-sTCP:LISTEN", "-P", "-n"])
            .output()
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 解析输出
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // 典型格式: command pid user fd type device size/off node name
            // node name 也就是 TCP *:80 (LISTEN)
            if parts.len() >= 9 {
                let program = parts[0];
                let pid = parts[1];
                let address_part = parts[8]; // *:8080

                if let Some(port_str) = address_part.split(':').last() {
                    let port = port_str.to_string();
                    ports.push(PortInfo {
                        pid: pid.to_string(),
                        port,
                        protocol: "TCP".to_string(),
                        program: program.to_string(),
                    });
                }
            }
        }
    }

    Ok(ports)
}

#[command]
pub fn kill_process(pid: String) -> Result<String, String> {
    if pid.is_empty() {
        return Err("PID cannot be empty".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = Command::new("taskkill")
            .args(&["/F", "/PID", &pid])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok("Process killed".to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("kill")
            .args(&["-9", &pid])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok("Process killed".to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}
