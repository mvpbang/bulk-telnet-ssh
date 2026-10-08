# bulk-telnet-ssh (Rust 重构版)

高性能批量网络策略连通性探测工具（基于 Rust 2021 + Tokio 异步运行时重构）。

本工具用于并发通过 SSH 登录一组源 Linux 主机（`ips`），并在各主机上并发探测目标网络端口（`target`）的连通性。

---

## 🚀 重构亮点与对比

| 特性维度 | 原版 (Go) | 重构版 (Rust) |
| :--- | :--- | :--- |
| **探测手段** | 依赖远程系统的 `telnet` 命令 | **彻底抛弃 telnet**！首选 **SSH direct-tcpip 协议通道**（毫秒级、零远程依赖），权限受限时自动降级为原生 **Bash `/dev/tcp`** |
| **远程依赖** | 必须安装 `telnet`（经常报错 `telnet not installed`） | **零远程依赖**，无须安装 telnet、nc、curl 等任何外部工具 |
| **连接复用** | 每个目标 × 每个源 IP 重复握手，耗时长且易触发 `reset by peer` | **单机长连接复用（Connection Reuse）**，每台源机仅 1 次握手建联，握手次数减少 90%+ |
| **并发控制** | 硬编码 `concurrency > 10` 强制退出，与配置冲突 | 基于 `tokio::sync::Semaphore` **弹性并发控制**，自由调节并发度 |
| **日志与统计** | 执行后扫文本重读 `ssh.log` 统计 | **实时流式终端彩色高亮** + **结构化表格汇总**，同时兼容历史 `ssh.log` 格式 |

---

## 📦 编译与运行

### 1. 编译二进制
```bash
cargo build --release
```
编译产物位于 `target/release/bulk-telnet-ssh`。

### 2. 命令行使用
```bash
# 使用默认配置 (ips.yml) 与默认输出 (ssh.log)
./target/release/bulk-telnet-ssh

# 自定义配置文件路径
./target/release/bulk-telnet-ssh -c my_ips.yml

# 临时调整并发度与超时时间（秒）
./target/release/bulk-telnet-ssh -c ips.yml -P 16 -t 5

# 查看帮助
./target/release/bulk-telnet-ssh --help
```

---

## ⚙️ 配置文件说明 (`ips.yml`)

100% 向后兼容现有 `ips.yml` 格式：

```yaml
# 默认 SSH 登录账户密码
auth:
    user: root
    password: your_password
    # ips 默认端口 (支持数字或字符串，如 22 或 "22")
    port: 22
    # target 并发控制数
    concurrency: 16

# 批量 SSH 登录源 IP 列表
ips:
    # 不写端口时默认读取 auth.port，存在端口则使用指定的端口
    - 172.20.189.75
    - 172.20.189.75:2222
    - 172.20.189.73:36000

# 测试端口连通性目标 (host:port)
target:
    - 172.20.189.75:22
    - 172.20.189.75:21
    - 172.20.189.75:23
    - 172.20.189.71:80
```

---

## 📊 状态标识与日志含义

### 终端实时标识与 `ssh.log` 对应关系
| 终端显示 | `ssh.log` 关键字 | 含义 |
| :--- | :--- | :--- |
| `[✓ 成功]` | `成功 ... pong ...` | **策略通**：目标端口连通测试通过 |
| `[✗ 拒绝]` | `失败 ... refused ...` | **策略通，端口未监听**：TCP 握手收到 RST 拒绝包 |
| `[✗ 超时]` | `失败 ... timed out ...` | **网络不通 / 超时**：防火墙 DROP 或路由丢包 |
| `[✗ 无路由]` | `失败 ... no route ...` | **主机不可达**：路由表中无到目标主机的路径 |
| `[! 登录失败]` | `login false ...` | **源主机 SSH 登录失败**：密码错误或握手失败（已自动重试 3 次） |
| `[! 异常]` | `失败+++ ...` | 其他未知异常信息 |

---

## 💡 探测原理解析

1. **首选引擎：SSH 2.0 Direct-TCPIP 原生通道**  
   本地客户端在与源主机认证成功的 SSH 连接中，直接向远程 sshd 请求 `direct-tcpip` 转发通道到目标 `target:port`。远程 sshd 会代替发起 TCP 三次握手：
   - 若建立成功，立即返回确认，直接判定为 `pong`；
   - 若端口关闭，返回连接拒绝，判定为 `refused`；
   - 若超时，本地定时器触发，判定为 `timed out`。  
   *整个过程无需在远程启动任何 shell 或派生任何进程，极速且免环境依赖。*

2. **兜底引擎：Bash `/dev/tcp`**  
   若部分受限主机在 `/etc/ssh/sshd_config` 中配置了 `AllowTcpForwarding no`，系统会自动捕获并无缝降级执行：
   ```bash
   timeout --signal=9 3 bash -c 'exec 3<>/dev/tcp/<target_host>/<target_port>' 2>&1
   ```
   依赖 Linux 内核与 Bash 的原生虚拟设备，彻底告别过时的 `telnet` 命令依赖。