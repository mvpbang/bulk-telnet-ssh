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

type Top struct {
	Auth   *Auth    `yaml:"auth"`
	IPs    []string `yaml:"ips"`
	Target []string `yaml:"target"`
}

type Auth struct {
	User     string `yaml:"user"`
	Password string `yaml:"password"`
}

// 全局变量
var config Top

func readYaml() {
	yamlFile, err := os.ReadFile("ips.yml")
	handleErr(err)

	// 解码
	err = yaml.Unmarshal(yamlFile, &config)
	handleErr(err)
}

// 测试联通性
func doTelnet(src string, target string, sem chan struct{}, wg *sync.WaitGroup) {
	defer func() {
		// 释放信号通告，允许新的会话加入
		<-sem
		wg.Done()
	}()
	// 从通道获取信号，满了则阻塞
	sem <- struct{}{}

	// Create a SSH client
	client, err := ssh.Dial("tcp", src, &ssh.ClientConfig{
		User: config.Auth.User,
		Auth: []ssh.AuthMethod{
			ssh.Password(config.Auth.Password),
		},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		Timeout:         3 * time.Second,
	})

	if err != nil {
		// login fail continue to next ip
		log.Printf("login false %s, err:%s\n: ", src, err)
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
	buf, _ := session.CombinedOutput(cmd)

	// 判断端口是否可以正常连接上
	switch {
	case strings.Contains(string(buf), "Killed"):
		log.Printf("失败 %s killed %s\n", src, target)
	case strings.Contains(string(buf), "refused"):
		log.Printf("失败 %s refused %s\n", src, target)
	case strings.Contains(string(buf), "timed out"):
		log.Printf("失败 %s timed out %s\n", src, target)
	// no  telnet
	case strings.Contains(string(buf), "bash") || strings.Contains(string(buf), "No such file"):
		log.Printf("失败 %s telnet not installed %s\n", src, target)
	// No route to
	case strings.Contains(string(buf), "route"):
		log.Printf("失败 %s no route %s\n", src, target)
	// pong
	case strings.Contains(string(buf), "^]"):
		log.Printf("成功 %s pong %s\n", src, target)
	// no match print
	default:
		log.Printf("失败+++ " + string(buf))
	}
}

func init() {
	logFile2, err := os.OpenFile("ssh.log", os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	handleErr(err)

	mw := io.MultiWriter(os.Stdout, logFile2)
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.SetOutput(mw)
}

func main() {
	readYaml()

	// linux default MaxSessions 8
	sem := make(chan struct{}, 8)

	// 并发执行
	var wg sync.WaitGroup

	for _, src := range config.IPs {
		for _, target := range config.Target {
			wg.Add(1)
			go doTelnet(src, target, sem, &wg)
		}
	}
	// wait...
	wg.Wait()
	close(sem)
}

func handleErr(err error) {
	if err != nil {
		log.Fatal("catch: ", err)
	}
}
