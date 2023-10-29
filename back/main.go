package main

import (
	"fmt"
	"golang.org/x/crypto/ssh"
	"gopkg.in/yaml.v3"
	"io"
	"io/ioutil"
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
	yamlFile, err := ioutil.ReadFile("ips.yml")
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
		//wgIp.Add(1)

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
			if strings.Count(string(buf), "Killed") > 0 {
				log.Printf("%s killed \n", target)
			} else if strings.Count(string(buf), "refused") > 0 {
				log.Printf("%s refused\n", target)
			} else if strings.Count(string(buf), "timed out") > 0 {
				log.Printf("%s telnet %s timed out\n", ip, target)
			} else if strings.Count(string(buf), "bash") > 0 {
				log.Printf("%s telnet not installed %s \n", ip, target)
			} else {
				log.Printf("%s telnet %s pong\n", ip, target)
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
	//loop
	var wg sync.WaitGroup
	for _, target := range config.Target {
		wg.Add(1)
		go TestPong(target, &wg)
		//fmt.Println("+++++++++++++", target)
	}
	// Wait for all login goroutines to finish
	wg.Wait()
}
