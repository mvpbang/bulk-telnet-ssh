package util

import (
	"golang.org/x/crypto/ssh"
	"log"
	"sync"
	"time"
)

func ProbeSsh(target string, wg *sync.WaitGroup) {
	defer wg.Done()

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

			//
			client.Dial("tcp", "")

		}(ip)
	}

	// Wait for all login goroutines to finish
	wgIp.Wait()
}
