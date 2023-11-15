# 目的
通过账户密码形式登陆到ssh服务器，在对远程的端口进行探测

# 步骤
- 根据系统类型、选择对应的版本
- 修改ips.yml相关配置
- 运行bulk-telnet
- 查看日志check.log

# 日志标识含义
|        日志关键字         |      含义      |
|:--------------------:|:------------:|
|     login false      |   ssh登陆失败    |
|        Killed        |   网络不通,超时    |
|       refused        |  策略通的，端口没监听  |
|      timed out       |      超时      |
| telnet not installed | telnet   未安装 |
|         pong         |  端口连通性，测试通过  |
 |          失败          |   telnet失败   |
|          成功          |   telnet成功   |
