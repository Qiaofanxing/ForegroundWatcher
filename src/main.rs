use std::thread::sleep;
use std::time::Duration;

use chrono::Local; // 用于获取和格式化当前时间
use log::{info, LevelFilter}; // 日志宏和日志级别过滤器
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode}; // 简单日志库，用于配置和初始化日志记录
use sysinfo::{Pid, ProcessesToUpdate, System}; // 系统信息库，用于获取进程信息
use windows::Win32::Foundation::HWND; // Windows句柄类型
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, // 获取当前活动窗口的句柄
    GetWindowThreadProcessId, // 获取窗口所属进程的ID
    GetWindowTextW, // 获取窗口标题的宽字符版本
    GetWindowTextLengthW, // 获取窗口标题的长度（宽字符）
};

// 配置日志记录，仅输出到控制台
fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    TermLogger::init(
        LevelFilter::Info, // 设置日志级别为Info
        Config::default(), // 使用默认的日志配置
        TerminalMode::Mixed, // 日志输出模式为混合模式（根据终端类型选择）
        ColorChoice::Auto, // 自动选择颜色显示
    )?;
    Ok(())
}

// 获取活动窗口句柄
fn get_active_window_handle() -> Option<HWND> {
    unsafe { GetForegroundWindow().into() } // 调用Windows API获取当前活动窗口句柄
}

// 获取窗口标题
fn get_window_text(hwnd: HWND) -> Option<String> {
    unsafe {
        let length = GetWindowTextLengthW(hwnd) + 1; // 获取窗口标题的长度，并加1以包含终止符
        if length == 0 {
            return None; // 如果长度为0，则没有标题
        }
        let mut buffer = vec![0u16; length as usize]; // 创建一个缓冲区存储宽字符标题
        let copied = GetWindowTextW(hwnd, &mut buffer); // 获取窗口标题
        if copied == 0 {
            return None; // 如果复制的字符数为0，表示获取失败
        }
        // 转换UTF-16为Rust字符串
        Some(
            String::from_utf16_lossy(&buffer[..copied as usize]) // 将UTF-16编码转换为Rust的String
                .trim_end_matches('\u{0}') // 去除末尾的空字符
                .to_string(),
        )
    }
}

// 获取进程ID
fn get_process_id(hwnd: HWND) -> Option<u32> {
    unsafe {
        let mut pid: u32 = 0; // 初始化进程ID
        GetWindowThreadProcessId(hwnd, Some(&mut pid)); // 获取窗口所属进程的ID
        if pid != 0 {
            Some(pid) // 如果获取到有效的PID，返回
        } else {
            None // 否则返回None
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    setup_logging()?;

    info!("程序启动"); // 记录程序启动信息

    let mut last_hwnd: Option<HWND> = None; // 存储上一个活动窗口的句柄，以检测窗口变化
    let mut system = System::new(); // 创建一个System对象，用于获取系统信息

    loop {
        if let Some(hwnd) = get_active_window_handle() { // 获取当前活动窗口句柄
            if Some(hwnd) != last_hwnd { // 检查是否与上一次的句柄不同，表示窗口发生变化
                last_hwnd = Some(hwnd); // 更新最后一个窗口句柄
                if let Some(pid_value) = get_process_id(hwnd) { // 获取窗口所属进程的ID
                    let pid = Pid::from(pid_value as usize); // 将u32类型的PID转换为sysinfo库的Pid类型
                    // 刷新特定进程的信息，第二个参数决定是否移除已经结束的进程
                    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
                    if let Some(process) = system.process(pid) { // 获取进程信息
                        let exe_path = process
                            .exe()
                            .map_or("未知路径".to_string(), |p| p.to_string_lossy().to_string()); // 获取可执行文件路径，如果不可用则标记为“未知路径”
                        let window_title = get_window_text(hwnd).map_or("未知窗口".to_string(), |title| title); // 获取窗口标题，如果获取失败则标记为“未知窗口”
                        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S"); // 获取当前时间并格式化
                        info!(
                            "{} | 进程ID: {} | 窗口标题: {} | 执行路径: {}",
                            timestamp, pid_value, window_title, exe_path
                        ); // 记录日志信息，包括时间、进程ID、窗口标题和执行路径
                    } else {
                        // 如果进程可能已经结束
                        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S"); // 获取当前时间并格式化
                        info!(
                            "{} | 进程ID: {} 不存在或已结束",
                            timestamp, pid_value
                        ); // 记录进程不存在或已结束的信息
                    }
                }
            }
        }
        sleep(Duration::from_millis(10)); // 休眠10毫秒，作为下次检查的间隔
    }
}