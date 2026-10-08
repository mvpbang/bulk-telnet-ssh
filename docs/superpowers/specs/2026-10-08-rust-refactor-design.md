# bulk-telnet-ssh Rust 重构设计文档

## 1. 背景与重构目标

`bulk-telnet-ssh` 原版为 Go 语言实现的运维网络策略批量连通性探测工具。用户通过本地配置文件 `ips.yml` 指定一批待 SSH 登录的源 Linux 主机（`ips`），以及一批需要探测的网络目标端口（`target`）。程序并发登录源主机并在其上探测目标网络端口连通性。

### 1.1 原版痛点
1. **网络连接未复用**：对每一个 `(target, ip)` 组合均独立执行一次完整的 SSH Dial 握手。在目标数或主机数稍多时产生大量的全握手请求，极易触发远端 Linux `MaxStartups` 限制与连接重置。
2. **远程探测手段单一脆弱**：原版依赖在远端调用 `telnet` 命令（`echo quit | timeout --signal=9 3 telnet host port`），但现代 Linux 生产环境普遍默认未安装 `telnet`，导致大量非网络原因的探测失败，且依赖终端控制字符正则匹配输出（如 `^]`）脆弱不稳定。
3. **并发限制硬编码冲突**：代码内硬编码 `concurrency > 10` 则退出程序，但默认配置文件配置了更高的并发数，导致逻辑冲突。
4. **统计汇总方式落后**：执行完毕后通过重新读取并逐行正则解析 `ssh.log` 文本文件获取结果，缺乏结构化结果汇总。

### 1.2 重构目标
1. **高性能与连接复用**：基于 Rust 与异步运行时重构。对每台源主机复用长连接会话（Connection Reuse），单主机只需 1 次 SSH 握手（带 3 次自动重试与超时控制）。
2. **丢弃 telnet，采用双引擎智能探测**：
   - **首选引擎（SSH Direct-TCPIP 原生协议通道）**：基于 SSH 2.0 协议标准 `direct-tcpip` 通道机制进行远端到目标的 TCP 三次握手测试。**零远程依赖、零进程开销、毫秒级响应**。
   - **兜底引擎（Bash 原生 `/dev/tcp`）**：针对远端 `sshd_config` 配置了 `AllowTcpForwarding no`（禁止端口转发）的主机，自动优雅降级为执行原生 `bash -c '</dev/tcp/{host}/{port}'`，彻底移除对 `telnet` 的依赖。
3. **弹性并发控制**：基于 `tokio::sync::Semaphore` 实现灵活的并发控制，支持用户自定义并发值且不会强退。
4. **日志与汇总体验升级**：输出兼容历史格式的 `ssh.log`，同时在终端打印结构化状态与美观的汇总统计表格。

---

## 2. 架构设计与工程布局

### 2.1 Git 分支策略
从 `main` 分支新建并检出 `refactor/rust` 分支，所有重构代码在该分支提交。

### 2.2 项目工程结构
```text
bulk-telnet-ssh/
├── Cargo.toml               # 项目清单与依赖配置
├── ips.yml                  # 配置文件（与旧版配置格式完全向后兼容）
├── README.md                # 现代化项目说明与原理解释
├── docs/
│   └── superpowers/specs/   # 架构与设计规范文档
├── src/
│   ├── main.rs              # 程序入口、CLI 参数解析、编排流转
│   ├── config.rs            # 配置读取、反序列化与字段校验
│   ├── error.rs             # 统一错误类型定义
│   ├── ssh/
│   │   ├── mod.rs
│   │   └── client.rs        # SSH 连接管理、复用、握手重试与鉴权
│   ├── probe/
│   │   ├── mod.rs
│   │   ├── engine.rs        # 探测引擎抽象与双引擎调度
│   │   ├── direct_tcp.rs    # Direct-TCPIP 协议级极速通道探测
│   │   └── bash_tcp.rs      # Bash /dev/tcp 原生探测（降级方案）
│   ├── runner.rs            # 异步任务调度器与 Semaphore 并发控制
│   └── reporter.rs          # 结构化统计、终端输出与 ssh.log 兼容写入
└── tests/
    └── integration_test.rs  # 核心模块集成与解析测试
```

---

## 3. 核心模块与实现细节

### 3.1 配置模块 (`config.rs`)
向下兼容现有 `ips.yml` 格式：
```yaml
auth:
  user: root
  password: secret
  port: 22
  concurrency: 16

ips:
  - 172.20.189.75
  - 172.20.189.75:2222

target:
  - 172.20.189.75:80
  - 172.20.189.71:443
```
- 支持 `auth.port` 为数字或字符串。
- 自动为没有携带端口的 `ips` 补充 `auth.port`。
- 校验 `ips` 与 `target` 格式（合法 IPv4/IPv6/域名与端口组合）。

### 3.2 SSH 连接管理模块 (`ssh/client.rs`)
- 使用 `ssh2` 库管理 SSHv2 会话。
- **连接重试与退避**：建立连接时支持最多 3 次握手重试，超时设为 3 秒。针对握手超时或网络重置提供退避重试。
- **连接复用**：构建 `SshHostManager`，每个源 IP 唯一对应一个保持连接的 `SshClient`。所有针对该源主机的目标探测任务均在该已认证连接上派生独立的 channel，消除重复认证与握手开销。

### 3.3 双引擎探测机制 (`probe/`)
对每个目标 `(target_host, target_port)` 执行连通性测试：
1. **Engine A: Direct-TCPIP 协议通道探测**
   - 调用 `session.channel_direct_tcpip(target_host, target_port, None)`。
   - 若返回通道句柄：说明远端与目标端口已完成 TCP 握手 $\rightarrow$ 状态为 `Pong`（成功），立即关闭通道释放资源。
   - 若返回错误为 `SSH_OPEN_CONNECT_FAILED` 或 Connection Refused：说明目标主机在线但端口未监听 $\rightarrow$ 状态为 `Refused`。
   - 若超时（本地设定 3 秒超时） $\rightarrow$ 状态为 `TimedOut`。
   - 若返回错误为 `SSH_OPEN_ADMINISTRATIVELY_PROHIBITED`（远端禁止转发）：无缝自动触发降级为 Engine B。
2. **Engine B: Bash `/dev/tcp` 原生探测**
   - 打开 session 并执行命令：
     ```bash
     timeout 3 bash -c 'exec 3<>/dev/tcp/<host>/<port>' 2>&1
     ```
   - 退出码 0 $\rightarrow$ 状态为 `Pong`。
   - 输出包含 `Connection refused` 或退出码 1 $\rightarrow$ 状态为 `Refused`。
   - 退出码 124 或输出包含 `timed out` / `Killed` $\rightarrow$ 状态为 `TimedOut`。
   - 输出包含 `No route` $\rightarrow$ 状态为 `NoRoute`。

### 3.4 任务调度与并发控制 (`runner.rs`)
- 输入源主机池与目标列表，生成笛卡尔积任务集。
- 采用 `tokio::sync::Semaphore` 作为全局并发令牌池，令牌数取 `config.auth.concurrency`（默认 8，若配置给定则按配置值，最小为 1）。
- 每个探测任务通过 `tokio::task::spawn_blocking` 处理底层通道 I/O，保证异步调度器高吞吐且不阻塞 runtime worker。

### 3.5 日志与报告汇总 (`reporter.rs`)
1. **日志文件写入 (`ssh.log`)**：
   - 保持旧版关键字格式兼容，追加写入：
     - `成功 172.20.189.75 pong 172.20.189.75:22`
     - `失败 172.20.189.75 refused 172.20.189.75:21`
     - `失败 172.20.189.75 timed out 172.20.189.75:23`
     - `login false 172.20.189.73:36000, err: ...`
2. **终端结构化控制台输出**：
   - 彩色实时任务状态打印。
   - 测试结束时在控制台展示对齐的表格化 Summary 报告：
     - 总探测点数、成功数、超时数、拒绝数、登录失败数。
     - 最终结论（“策略全部检测通过” / “策略部分成功，请检查失败的日志” / “存在错误请检查日志”）。

---

## 4. 异常处理与边缘场景

1. **源主机不可达 / 认证失败**：
   重试 3 次后若依然失败，标记该源主机为不可用，所有以该主机为源的探测任务直接标记为 `LoginFailed`，无需空耗超时时间。
2. **目标主机格式非法**：
   配置解析阶段前置校验，遇到无法识别的 `host:port` 格式立即输出友好告警。
3. **并发数过大保护**：
   若配置并发数极大（如大于 100），系统正常调度而不发生崩溃，同时保障安全优雅退出。

---

## 5. 测试与验证策略

1. **单元测试 (`cargo test`)**：
   - 配置反序列化解析测试（涵盖数字端口、字符串端口、默认端口补齐）。
   - 主机与端口拆分解析测试。
   - 错误状态码与输出文本正则映射测试。
2. **功能与冒烟测试**：
   - 本地回环或真实环境配置文件执行测试。
   - 校验生成的 `ssh.log` 格式与汇总输出。
