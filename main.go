package main

import (
	"bulk-telnet/util"
	"time"
)

func main() {
	util.ReadYaml()
	// window->linux telnet
	//util.ProbeLocal(util.Config.LocalTarget)

	//util.ProbeSsh(util.Config.SshTarget)

	/*	//loop
		var wg sync.WaitGroup
		for _, target := range util.Config.SshTarget {
			wg.Add(1)
			go util.ProbeSsh(target, &wg)
			//fmt.Println("+++++++++++++", target)
		}
		// Wait for all login goroutines to finish
		wg.Wait()*/

	for _, target := range util.Config.SshTarget {
		go util.ProbeSsh(target)
	}
	time.Sleep(10e9)
}
