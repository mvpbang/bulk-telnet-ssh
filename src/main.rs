use bulk_telnet_ssh::config::Config;
use bulk_telnet_ssh::reporter::Reporter;
use bulk_telnet_ssh::runner::TaskRunner;
use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "bulk-telnet-ssh",
    version = "0.2.0",
    about = "High-performance bulk network reachability probe over SSH (Rust refactored)"
)]
struct Args {
    /// 配置文件路径 (默认: ips.yml)
    #[arg(short, long, default_value = "ips.yml")]
    config: PathBuf,

    /// 日志输出文件路径 (默认: ssh.log)
    #[arg(short, long, default_value = "ssh.log")]
    log: PathBuf,

    /// 覆盖配置文件中的并发数限制
    #[arg(short = 'P', long)]
    concurrency: Option<usize>,

    /// SSH 连接与端口探测超时时间（秒）
    #[arg(short, long, default_value_t = 3)]
    timeout: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // 1. 加载配置文件
    let config = match Config::load_from_file(&args.config) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!(
                "{} 无法读取配置文件 '{}': {}",
                "【错误】".red().bold(),
                args.config.display(),
                err
            );
            process::exit(1);
        }
    };

    let concurrency = args.concurrency.unwrap_or(config.auth.concurrency).max(1);

    println!("{}", "=".repeat(60).cyan());
    println!(
        "{} {}",
        "bulk-telnet-ssh (Rust)".green().bold(),
        "v0.2.0 - 批量网络策略连通性探测工具".dimmed()
    );
    println!(
        "配置: {} | 源主机数: {} | 目标数: {} | 并发: {} | 超时: {}s",
        args.config.display().to_string().yellow(),
        config.ips.len().to_string().cyan(),
        config.target.len().to_string().cyan(),
        concurrency.to_string().green(),
        args.timeout.to_string().green()
    );
    println!("探测引擎: SSH direct-tcpip (首选) + Bash /dev/tcp (兜底)");
    println!("{}\n", "=".repeat(60).cyan());

    // 2. 初始化 Reporter 与 TaskRunner
    let reporter = Arc::new(Reporter::new(&args.log));
    let runner = TaskRunner::new(config)
        .with_concurrency(concurrency)
        .with_timeout(args.timeout);

    let reporter_clone = reporter.clone();
    let results = match runner
        .run_with_callback(move |res| {
            reporter_clone.record(&res);
        })
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("{} 执行任务时发生致命错误: {}", "【错误】".red().bold(), err);
            process::exit(1);
        }
    };

    // 3. 打印最终汇总统计报表
    reporter.print_summary(&results);

    // 4. 判断退出码
    let all_passed = results.iter().all(|r| r.status.is_success());
    if !all_passed && !results.is_empty() {
        process::exit(1);
    }
}
