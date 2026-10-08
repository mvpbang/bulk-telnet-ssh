use crate::config::Config;
use crate::error::AppError;
use crate::probe::{probe_target, ProbeResult, ProbeStatus};
use crate::ssh::SshClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};

pub struct TaskRunner {
    pub config: Config,
    pub timeout_secs: u64,
    pub concurrency: usize,
}

impl TaskRunner {
    pub fn new(config: Config) -> Self {
        let concurrency = config.auth.concurrency.max(1);
        Self {
            config,
            timeout_secs: 3,
            concurrency,
        }
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        if concurrency > 0 {
            self.concurrency = concurrency;
        }
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        if timeout_secs > 0 {
            self.timeout_secs = timeout_secs;
        }
        self
    }

    /// 执行全部探测任务，并通过回调函数在每个结果产生时即时通知
    pub async fn run_with_callback<F>(&self, on_result: F) -> Result<Vec<ProbeResult>, AppError>
    where
        F: Fn(ProbeResult) + Send + Sync + 'static,
    {
        let resolved_ips = self.config.resolved_ips()?;
        let resolved_targets = self.config.resolved_targets()?;

        let on_result = Arc::new(on_result);
        let mut results = Vec::new();

        // 阶段一：并发连接各源主机（每台源主机仅建联 1 次）
        let user = self.config.auth.user.clone();
        let pass = self.config.auth.password.clone();
        let timeout = Duration::from_secs(self.timeout_secs);

        let mut connect_tasks = Vec::new();
        for (host, port) in resolved_ips {
            let u = user.clone();
            let p = pass.clone();
            let h = host.clone();
            connect_tasks.push(tokio::spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    SshClient::connect(&h, port, &u, &p, timeout, 3)
                })
                .await;
                match res {
                    Ok(client_res) => (host, port, client_res),
                    Err(join_err) => (
                        host,
                        port,
                        Err(AppError::Ssh(format!("Task spawn error: {}", join_err))),
                    ),
                }
            }));
        }

        let mut active_clients = HashMap::new();
        for task in connect_tasks {
            let (host, port, res) = task.await.map_err(|e| AppError::Ssh(e.to_string()))?;
            let addr = format!("{}:{}", host, port);
            match res {
                Ok(client) => {
                    active_clients.insert(addr, Arc::new(client));
                }
                Err(err) => {
                    // 若源机登录失败，针对所有目标记录登录失败
                    for (t_host, t_port) in &resolved_targets {
                        let target_addr = format!("{}:{}", t_host, t_port);
                        let fail_res = ProbeResult {
                            src: addr.clone(),
                            target: target_addr,
                            status: ProbeStatus::SshLoginFailed(err.to_string()),
                            duration_ms: 0,
                        };
                        on_result(fail_res.clone());
                        results.push(fail_res);
                    }
                }
            }
        }

        // 阶段二：受 Semaphore 限制并发探测
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let (tx, mut rx) = mpsc::channel::<ProbeResult>(100);

        // 启动后台接收器收集结果
        let on_result_collector = on_result.clone();
        let collector_handle = tokio::spawn(async move {
            let mut collected = Vec::new();
            while let Some(res) = rx.recv().await {
                on_result_collector(res.clone());
                collected.push(res);
            }
            collected
        });

        let mut probe_handles = Vec::new();
        for (target_host, target_port) in &resolved_targets {
            for client in active_clients.values() {
                let client = client.clone();
                let t_host = target_host.clone();
                let t_port = *target_port;
                let sem = sem.clone();
                let tx = tx.clone();
                let timeout_secs = self.timeout_secs;

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire_owned().await.expect("semaphore closed");
                    let host_clone = t_host.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        probe_target(&client, &host_clone, t_port, timeout_secs)
                    })
                    .await
                    .unwrap_or_else(|join_err| ProbeResult {
                        src: "unknown".to_string(),
                        target: format!("{}:{}", t_host, t_port),
                        status: ProbeStatus::Error(format!("Task execution failed: {}", join_err)),
                        duration_ms: 0,
                    });
                    let _ = tx.send(res).await;
                });
                probe_handles.push(handle);
            }
        }

        // 等待所有探测任务派发完成
        for h in probe_handles {
            let _ = h.await;
        }

        // 关闭发送端通道，等待收集完成
        drop(tx);
        let mut probe_results = collector_handle
            .await
            .map_err(|e| AppError::Ssh(e.to_string()))?;
        results.append(&mut probe_results);

        Ok(results)
    }

    pub async fn run(&self) -> Result<Vec<ProbeResult>, AppError> {
        self.run_with_callback(|_| {}).await
    }
}
