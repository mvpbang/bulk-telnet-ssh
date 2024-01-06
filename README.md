# 需求
```
1、并发登录linux
2、并发在linux上测试连通性，支持自定义控制并发
3、支持并发ips自定义默认端口
```

# ips.yml
> 配置文件

```yaml
# 默认ssh登录账户密码
auth:
    user: bang
    password: 321321
    #ips默认端口
    port: 22
    #target并发
    concurrency: 8

# 批量ssh登录ip
ips:
    #不写端口默认读取auth.port,存在则使用提供的端口
    -  172.20.189.75
    -  172.20.189.75:2222
    -  172.20.189.75:2323
    -  172.20.189.73:36000

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
- 查看日志ssh.log

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

# troubleshooting 

```
//错误日志
1ogin false x:36008,err:ssh: handshake falled: read tcp y:51754-z:36000: read: connection reset by pee

//解决办法
降低并发数ips.yml auth.concurrency: 6
```