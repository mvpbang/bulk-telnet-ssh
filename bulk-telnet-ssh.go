package main

import (
	"bufio"
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

func do(target string, sem chan int, wg *sync.WaitGroup) {
	defer wg.Done()
	//for .. ips
	for _, src := range config.IPs {
		//ipaddr = host:port
		var ipaddr string
		if strings.Contains(src, ":") {
			ipaddr = src
		} else {
			ipaddr = src + ":" + config.Auth.Port
		}
		// Create a SSH client
		client, err := ssh.Dial("tcp", ipaddr, &ssh.ClientConfig{
			User: config.Auth.User,
			Auth: []ssh.AuthMethod{
				ssh.Password(config.Auth.Password),
			},
			HostKeyCallback: ssh.InsecureIgnoreHostKey(),
			Timeout:         3 * time.Second,
		})
		// print and next
		if err != nil {
			// login fail continue to next ip
			log.Printf("login false %s, err:%s\n: ", ipaddr, err)
		} else {
			// bulk session and limit telnet by semaphore
			sem <- 1
			go doTelnet(client, ipaddr, target, sem)
		}
	}
}

func doTelnet(client *ssh.Client, src string, target string, sem chan int) {
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
	tHost := strings.Split(target, ":")[0]
	tPort := strings.Split(target, ":")[1]
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
		log.Printf("失败+++ %s->%s: "+string(buf), src, target)
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
	readYaml()

	// linux default MaxSessions 8
	sem := make(chan int, config.Auth.Concurrency)

	// parallel do
	var wg sync.WaitGroup
	for _, target := range config.Target {
		wg.Add(1)
		go do(target, sem, &wg)
	}
	// wait...
	wg.Wait()
	for {
		if len(sem) == 0 {
			break
		}
	}
	defer printFail()
}

func printFail() {
	f, err := os.Open("ssh.log")
	handleErr(err)
	defer f.Close()
	fmt.Println("汇总信息：", strings.Repeat("+", 20))
	count := 0
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		if strings.Contains(scanner.Text(), "失败") {
			count++
			fmt.Println(scanner.Text())
		}
	}
	if count == 0 {
		fmt.Println("策略全部检测通过！！！：")
	}
}

func handleErr(err error) {
	if err != nil {
		log.Fatal("catch: ", err)
	}
}
