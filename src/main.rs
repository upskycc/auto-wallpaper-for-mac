mod cache;
mod config;
mod install;
mod log;
mod paths;
mod power;
mod scheduler;
mod sources;
mod state;
mod wallpaper;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::config::{resolve_config_path, write_example, Config};
use crate::install::install;
use crate::paths::default_config_path;
use crate::scheduler::{rotate_once, run_loop};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            crate::log::warn(&err);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut config_path: Option<PathBuf> = None;
    let mut once = false;
    let mut do_install = false;
    let mut write_config = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                config_path = Some(PathBuf::from(args.get(i).ok_or("--config 需要路径")?));
            }
            "--once" => once = true,
            "--install" => do_install = true,
            "--write-config" => write_config = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("未知参数: {other}")),
        }
        i += 1;
    }

    if write_config {
        let path = resolve_config_path(config_path);
        write_example(&path)?;
        println!("已写入 {}", path.display());
        return Ok(());
    }

    if do_install {
        return install();
    }

    let path = resolve_config_path(config_path);
    if !path.exists() {
        write_example(&path)?;
        crate::log::info(&format!("已创建默认配置 {}", path.display()));
    }
    let config = Config::load(&path)?;
    if once {
        rotate_once(&config);
        return Ok(());
    }
    crate::log::info(&format!("Wallflow 启动，配置 {}", path.display()));
    run_loop(&path);
    Ok(())
}

fn print_help() {
    println!(
        "wallflow — macOS 配置文件驱动的自动换壁纸

用法:
  wallflow                     常驻运行
  wallflow --once              立即更换一张
  wallflow --install           安装 LaunchAgent
  wallflow --write-config      写出示例配置
  wallflow --config <path>     指定配置文件

默认配置: {}
日志: ~/Library/Logs/wallflow.log",
        default_config_path().display()
    );
}
