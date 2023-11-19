package main

import (
	"fmt"
	"golang.org/x/crypto/ssh"
	"gopkg.in/yaml.v3"
	"io"
	"log"
	"os"
	"strings"
	"sync"
	"time"
)

// 定义yaml结构体

type Top struct {
	Auth   *Auth    `yaml:"auth"`
	IPs    []string `yaml:"ips"`
	Target []string `yaml:"target"`
}

type Auth struct {
	User     string `yaml:"user"`
	Password string `yaml:"password"`
}

var config Top

func ReadYaml() {
	// 读取yml
	yamlFile, err := os.ReadFile("ips.yml")
	if err != nil {
		log.Println(err)
		return
	}

	// 解码
	err = yaml.Unmarshal(yamlFile, &config)
	if err != nil {
		log.Println(err)
		return
	}
}

//测试联通性

func TestPong(target string, wg *sync.WaitGroup) {
	defer wg.Done()

	var wgIp sync.WaitGroup
	wgIp.Add(len(config.IPs))

	for _, ip := range config.IPs {
		go func(ip string) {
			defer wgIp.Done()

			// Create a SSH client
			client, err := ssh.Dial("tcp", ip, &ssh.ClientConfig{
				User: config.Auth.User,
				Auth: []ssh.AuthMethod{
					ssh.Password(config.Auth.Password),
				},
				HostKeyCallback: ssh.InsecureIgnoreHostKey(),
				Timeout:         3 * time.Second,
			})

			if err != nil {
				//登录失败跳出，继续下次执行
				log.Printf("login false %s, err:%s\n: ", ip, err)
				return
			}
			defer client.Close()

			// 创建会话
			session, err := client.NewSession()
			if err != nil {
				log.Println(err)
				return
			}
			defer session.Close()

			// 测试连通性
			tHost := strings.Split(target, ":")[0]
			tPort := strings.Split(target, ":")[1]
			cmd := fmt.Sprintf("echo quit | timeout --signal=9 6 telnet %s %s", tHost, tPort)

			buf, err := session.CombinedOutput(cmd)
			if err != nil {
				//log.Println(err)
			}
			//log.Println(string(buf))

			// 判断端口是否可以正常连接上
			switch {
			case strings.Contains(string(buf), "Killed"):
				log.Printf("失败 %s killed %s\n", ip, target)
			case strings.Contains(string(buf), "refused"):
				log.Printf("失败 %s refused %s\n", ip, target)
			case strings.Contains(string(buf), "timed out"):
				log.Printf("失败 %s timed out %s\n", ip, target)
			case strings.Contains(string(buf), "bash") || strings.Contains(string(buf), "No such file or directory"):
				log.Printf("失败 %s telnet not installed %s\n", ip, target)
			// pong
			case strings.Contains(string(buf), "^]"):
				log.Printf("成功 %s pong %s\n", ip, target)
			// no match print
			default:
				log.Println("失败 " + string(buf))
			}
		}(ip)
	}

	// Wait for all login goroutines to finish
	wgIp.Wait()
}

func init() {
	logFile2, err := os.OpenFile(".//check.log", os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		log.Panicln(err)
	}
	mw := io.MultiWriter(os.Stdout, logFile2)
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.SetOutput(mw)
}

func main() {
	ReadYaml()
	// linux default MaxSessions 10
	if len(config.Target) > 10 {
		log.Println("target more than 10, please less than 10")
		return
	}
	// loop
	var wg sync.WaitGroup
	for _, target := range config.Target {
		wg.Add(1)
		go TestPong(target, &wg)
	}
	// Wait for all login goroutines to finish
	wg.Wait()
}
