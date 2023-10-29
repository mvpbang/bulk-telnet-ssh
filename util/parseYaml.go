package util

import (
	"gopkg.in/yaml.v3"
	"log"
	"os"
)

// 定义ips-new.yml结构体

type Top struct {
	ProbeType   string    `yaml:"probe_type"`
	LocalTarget []string  `yaml:"local_target"`
	SshLogin    *SshLogin `yaml:"ssh_login"`
	SshTarget   []string  `yaml:"ssh_target"`
}

type SshLogin struct {
	Auth  *Auth    `yaml:"auth"`
	Hosts []string `yaml:"host"`
}

type Auth struct {
	User     string `yaml:"user"`
	Password string `yaml:"password"`
}

var Config Top

func ReadYaml() {
	// 读取yml
	yamlFile, err := os.ReadFile("ips-new.yml")
	if err != nil {
		log.Println(err)
		return
	}

	// 解码
	err = yaml.Unmarshal(yamlFile, &Config)
	if err != nil {
		log.Println(err)
		return
	}
}
