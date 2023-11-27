# 需求
```
1、并发登录linux
2、并发在linux上测试连通性(控制性并发，default 8)

//ips.yml
# 默认ssh登录账户密码
auth:
    user: bang
    password: 321321

# 批量ssh登录ip
ips:
    -  172.20.189.75:22
    -  172.20.189.75:22
    -  172.20.189.75:22
    -  172.20.189.73:22

# 测试端口连通性
target:
    -  172.20.189.75:22
    -  172.20.189.75:21
    -  172.20.189.75:23
    -  172.20.189.71:23
```

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
