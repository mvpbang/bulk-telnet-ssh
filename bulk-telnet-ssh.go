package main

import (
	"bufio"
	"fmt"
	"golang.org/x/crypto/ssh"
	"gopkg.in/yaml.v3"
	"io"
	"log"
	"net"
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
	User        string `yaml:"user"`
	Password    string `yaml:"password"`
	Port        string `yaml:"port"`
	Concurrency int    `yaml:"concurrency"`
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

func do(target string, sem chan struct{}, targetWg *sync.WaitGroup, wg *sync.WaitGroup) {
	defer wg.Done()

	tHost, tPort, err := net.SplitHostPort(target)
	if err != nil {
		log.Printf("失败 %s target format invalid: %v\n", target, err)
		return
	}

	//for .. ips
	for _, src := range config.IPs {
		//ipaddr = host:port
		var ipaddr string
		if strings.Contains(src, ":") {
			ipaddr = src
		} else {
			ipaddr = src + ":" + config.Auth.Port
		}

		var client *ssh.Client
		for retryCount := 0; retryCount < 3; retryCount++ {
			// Create a SSH client
			client, err = ssh.Dial("tcp", ipaddr, &ssh.ClientConfig{
				User: config.Auth.User,
				Auth: []ssh.AuthMethod{
					ssh.Password(config.Auth.Password),
				},
				HostKeyCallback: ssh.InsecureIgnoreHostKey(),
				Timeout:         3 * time.Second,
			})

			if err == nil {
				break
			}

			if !strings.Contains(err.Error(), "handshake") {
				break
			}

			sep := strings.Repeat("*", 20)
			fmt.Println(sep, "发现handshake进入retry:", err)
			time.Sleep(3 * time.Second)
		}

		if err != nil {
			// login fail continue to next ip
			log.Printf("login false %s, err:%s\n", ipaddr, err)
			continue
		}

		// bulk session and limit telnet by semaphore
		targetWg.Add(1)
		sem <- struct{}{}
		go doTelnet(client, ipaddr, tHost, tPort, sem, targetWg)
	}
}

func doTelnet(client *ssh.Client, src string, tHost string, tPort string, sem chan struct{}, wg *sync.WaitGroup) {
	defer wg.Done()
	defer func() {
		// free semaphore, allow next
		<-sem
	}()
	defer client.Close()

	// create session
	session, err := client.NewSession()
	if err != nil {
		log.Println(err)
		return
	}
	defer session.Close()

	// test pong
	target := net.JoinHostPort(tHost, tPort)
	cmd := fmt.Sprintf("echo quit | timeout --signal=9 3 telnet %s %s", tHost, tPort)
	buf, _ := session.CombinedOutput(cmd)
	// check pong
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
		log.Printf("失败+++ %s->%s: %s\n", src, target, string(buf))
	}
}

func init() {
	logFile2, err := os.OpenFile("ssh.log", os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	handleErr(err)

	//multi writer
	mw := io.MultiWriter(os.Stdout, logFile2)
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.SetOutput(mw)
}

func main() {
	defer printFail()

	readYaml()
	// linux default MaxSessions 8
	if config.Auth.Concurrency > 10 {
		fmt.Println("Auth.Concurrency 不能大于10")
		return
	}
	sem := make(chan struct{}, config.Auth.Concurrency)
	telnetWg := sync.WaitGroup{}
	// parallel do
	var wg sync.WaitGroup
	for _, target := range config.Target {
		wg.Add(1)
		go do(target, sem, &telnetWg, &wg)
	}
	// wait...
	wg.Wait()
	telnetWg.Wait()

}

func printFail() {
	f, err := os.Open("ssh.log")
	handleErr(err)
	defer f.Close()

	fmt.Println(strings.Repeat("+", 20), "汇总信息：", strings.Repeat("+", 20))
	fcount := 0
	ocount := 0
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		info := scanner.Text()
		if strings.Contains(info, "失败") {
			fcount++
			fmt.Println(info)
		}
		if !strings.Contains(info, "失败") && !strings.Contains(info, "成功") {
			ocount++
			fmt.Println(info)
		}
	}
	switch {
	case fcount == 0 && ocount == 0:
		fmt.Println("策略全部检测通过")
	case ocount != 0:
		fmt.Println("存在错误请检查日志")
	default:
		fmt.Println("策略部分成功，请检查失败的日志")
	}
}

func handleErr(err error) {
	if err != nil {
		log.Fatal("catch: ", err)
	}
}
