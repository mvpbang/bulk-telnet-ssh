package main

import (
	"bulk-telnet/util"
	"sync"
)

/*func init() {
	logFile2, err := os.OpenFile(".//check.log", os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		log.Panicln(err)
	}
	mw := io.MultiWriter(os.Stdout, logFile2)
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.SetOutput(mw)
}*/

/*func main() {
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
*/

/*func main() {
	util.ReadYaml()
	fmt.Println(util.Config.ProbeType)
	fmt.Println(util.Config.LocalTarget)
	fmt.Println(util.Config.SshLogin.Auth.User)
	fmt.Println(util.Config.SshTarget)

}
*/

func main() {
	util.ReadYaml()
	//loop
	var wg sync.WaitGroup
	for _, target := range util.Config.LocalTarget {
		wg.Add(1)
		go util.ProbeLocal(target, &wg)
		//fmt.Println("+++++++++++++", target)
	}
	// Wait for all login goroutines to finish
	wg.Wait()
}
