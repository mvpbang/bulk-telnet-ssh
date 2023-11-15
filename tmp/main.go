package main

import "go.uber.org/zap"

func main() {
	logger, _ := zap.NewProduction()
	defer logger.Sync()
	logger.Info("Hello World!")
	x := "gaga"
	logger.Sugar().Infof("%s gaga", x)
}
