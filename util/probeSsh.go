package util

import (
	"golang.org/x/crypto/ssh"
	"log"
	"sync"
	"time"
)

func ProbeSsh(target string) {
	var wgIp sync.WaitGroup
	wgIp.Add(len(Config.SshLogin.Hosts))

	// login && probe
	for _, ip := range Config.SshLogin.Hosts {
		//wgIp.Add(1)

		go func(ip string) {
			defer wgIp.Done()

			// Create a SSH client
			client, err := ssh.Dial("tcp", ip, &ssh.ClientConfig{
				User: Config.SshLogin.Auth.User,
				Auth: []ssh.AuthMethod{
					ssh.Password(Config.SshLogin.Auth.Password),
				},
				HostKeyCallback: ssh.InsecureIgnoreHostKey(),
				Timeout:         3 * time.Second,
			})

			if err != nil {
				// login fail, continue next
				log.Printf("login false %s, err:%s\n: ", ip, err)
				return
			}
			defer client.Close()

			// probe
			conn, err := client.Dial("tcp", target)
			if conn == nil {
				log.Printf("ok %s %s \n", Config.ProbeType, target)
				defer conn.Close()

			} else {
				log.Printf("false %s %s \n", Config.ProbeType, err)
			}
		}(ip)
	}

	// Wait for all login goroutines to finish
	wgIp.Wait()
}
