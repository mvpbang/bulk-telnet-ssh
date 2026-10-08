use crate::probe::{ProbeResult, ProbeStatus};
use chrono::Local;
use colored::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Reporter {
    log_path: PathBuf,
    file_mutex: Mutex<()>,
}

pub fn format_legacy_log(res: &ProbeResult) -> String {
    let ts = Local::now().format("%Y/%m/%d %H:%M:%S");
    match &res.status {
        ProbeStatus::Pong => format!("{} 成功 {} pong {}\n", ts, res.src, res.target),
        ProbeStatus::Refused => format!("{} 失败 {} refused {}\n", ts, res.src, res.target),
        ProbeStatus::TimedOut => format!("{} 失败 {} timed out {}\n", ts, res.src, res.target),
        ProbeStatus::NoRoute => format!("{} 失败 {} no route {}\n", ts, res.src, res.target),
        ProbeStatus::SshLoginFailed(err) => {
            format!("{} login false {}, err:{}\n", ts, res.src, err)
        }
        ProbeStatus::Error(err) => {
            format!("{} 失败+++ {}->{}: {}\n", ts, res.src, res.target, err)
        }
    }
}

impl Reporter {
    pub fn new<P: AsRef<Path>>(log_path: P) -> Self {
        Self {
            log_path: log_path.as_ref().to_path_buf(),
            file_mutex: Mutex::new(()),
        }
    }

    /// 实时记录一条探测结果（输出到控制台并追加写入文件）
    pub fn record(&self, result: &ProbeResult) {
        // 1. 终端彩色输出
        let badge = match &result.status {
            ProbeStatus::Pong => result.status.display_badge().green().bold(),
            ProbeStatus::Refused => result.status.display_badge().yellow(),
            ProbeStatus::TimedOut => result.status.display_badge().red(),
            ProbeStatus::NoRoute => result.status.display_badge().red(),
            ProbeStatus::SshLoginFailed(_) => result.status.display_badge().red().bold(),
            ProbeStatus::Error(_) => result.status.display_badge().magenta().bold(),
        };

        let duration_info = if result.duration_ms > 0 {
            format!("({}ms)", result.duration_ms)
        } else {
            String::new()
        };

        let detail_info = match &result.status {
            ProbeStatus::Pong => "pong".green(),
            ProbeStatus::Refused => "refused".yellow(),
            ProbeStatus::TimedOut => "timed out".red(),
            ProbeStatus::NoRoute => "no route".red(),
            ProbeStatus::SshLoginFailed(err) => format!("login failed: {}", err).red(),
            ProbeStatus::Error(err) => format!("error: {}", err).magenta(),
        };

        println!(
            "{} {} -> {} {} {}",
            badge, result.src.cyan(), result.target.bold(), detail_info, duration_info.dimmed()
        );

        // 2. 写入文件（保持原版日志格式兼容）
        let log_line = format_legacy_log(result);
        let _guard = self.file_mutex.lock().unwrap();
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    }

    /// 任务全部完成时打印汇总统计报表
    pub fn print_summary(&self, results: &[ProbeResult]) {
        let total = results.len();
        let mut success = 0;
        let mut refused = 0;
        let mut timed_out = 0;
        let mut no_route = 0;
        let mut login_fail = 0;
        let mut error_count = 0;

        let mut failures = Vec::new();

        for r in results {
            match &r.status {
                ProbeStatus::Pong => success += 1,
                ProbeStatus::Refused => {
                    refused += 1;
                    failures.push(r);
                }
                ProbeStatus::TimedOut => {
                    timed_out += 1;
                    failures.push(r);
                }
                ProbeStatus::NoRoute => {
                    no_route += 1;
                    failures.push(r);
                }
                ProbeStatus::SshLoginFailed(_) => {
                    login_fail += 1;
                    failures.push(r);
                }
                ProbeStatus::Error(_) => {
                    error_count += 1;
                    failures.push(r);
                }
            }
        }

        let sep = "+".repeat(20);
        println!("\n{} 汇总信息： {}", sep.cyan(), sep.cyan());
        println!(
            "总探测任务: {} | 成功: {} | 拒绝: {} | 超时: {} | 无路由: {} | 登录失败: {} | 异常: {}",
            total.to_string().bold(),
            success.to_string().green().bold(),
            refused.to_string().yellow(),
            timed_out.to_string().red(),
            no_route.to_string().red(),
            login_fail.to_string().red().bold(),
            error_count.to_string().magenta()
        );

        if !failures.is_empty() {
            println!("\n{}", "【未通过清单】:".yellow().bold());
            for f in &failures {
                let status_desc = match &f.status {
                    ProbeStatus::Refused => "失败 refused",
                    ProbeStatus::TimedOut => "失败 timed out",
                    ProbeStatus::NoRoute => "失败 no route",
                    ProbeStatus::SshLoginFailed(e) => &format!("login false, err: {}", e),
                    ProbeStatus::Error(e) => &format!("失败+++ {}", e),
                    _ => "",
                };
                println!("  - {} -> {}: {}", f.src, f.target, status_desc);
            }
        }

        println!("\n{}", "----------------------------------------".dimmed());
        if success == total && total > 0 {
            println!("{}", "策略全部检测通过".green().bold());
        } else if login_fail > 0 || error_count > 0 {
            println!("{}", "存在错误请检查日志".red().bold());
        } else {
            println!("{}", "策略部分成功，请检查失败的日志".yellow().bold());
        }
        println!("详细日志已追加至: {}\n", self.log_path.display().to_string().cyan());
    }
}
