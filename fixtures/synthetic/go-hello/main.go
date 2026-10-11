package main

import (
	"fmt"
	"runtime"
)

func main() {
	fmt.Printf("hello from go-hello built with %s\n", runtime.Version())
}
