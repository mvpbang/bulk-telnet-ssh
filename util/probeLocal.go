package util

//测试联通性
import (
	"fmt"
	"net"
	"sync"
	"time"
)

func ProbeLocal(localTarget []string) {
	var wgIp sync.WaitGroup
	wgIp.Add(len(localTarget))

	for _, ip := range localTarget {
		go func(ip string) {
			defer wgIp.Done()
			conn, err := net.DialTimeout("tcp", ip, 3*time.Second)
			if err == nil {
				defer conn.Close()
				fmt.Printf("ok %s %s \n", Config.ProbeType, ip)

			} else {
				fmt.Printf("false %s %s \n", Config.ProbeType, err)
			}
		}(ip)
	}
	// Wait for all login goroutines to finish
	wgIp.Wait()
}
