// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 初始化日志系统
    #[cfg(debug_assertions)]
    env_logger::init();

    // 打印版本信息
    #[cfg(debug_assertions)]
    println!("Delo 重复图像检测工具 v{}", env!("CARGO_PKG_VERSION"));
    
    // 启动应用
    delo_lib::run()
}