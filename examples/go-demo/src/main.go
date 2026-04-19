package main

import "fmt"

func main() {
	fmt.Println("hello from the go demo")
	greet("world")
}

func greet(name string) {
	// TODO: switch to structured logger
	fmt.Println("hi,", name)
}
