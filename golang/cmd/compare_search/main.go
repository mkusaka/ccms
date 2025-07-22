package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mkusaka/ccms/golang/internal/schemas"
	"github.com/mkusaka/ccms/golang/internal/search"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: compare_search <query>")
		os.Exit(1)
	}

	query := strings.Join(os.Args[1:], " ")
	
	// Find one test file
	home, _ := os.UserHomeDir()
	pattern := filepath.Join(home, ".claude", "projects", "**", "*.jsonl")
	files, _ := filepath.Glob(pattern)
	
	if len(files) == 0 {
		fmt.Println("No files found")
		return
	}

	// Test with first file
	testFile := files[0]
	fmt.Printf("Testing with file: %s\n", filepath.Base(testFile))
	
	// Method 1: Original simple message approach
	fmt.Println("\n=== Method 1: SimpleMessage ===")
	messages1, err := search.LoadSimpleMessages(testFile)
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		return
	}
	
	count1 := 0
	for _, msg := range messages1 {
		content := msg.GetContentText()
		if strings.Contains(strings.ToLower(content), strings.ToLower(query)) {
			count1++
			if count1 <= 3 {
				fmt.Printf("Type: %s, Content preview: %.50s...\n", msg.GetType(), content)
			}
		}
	}
	fmt.Printf("Total matches: %d\n", count1)
	
	// Method 2: Line by line checking
	fmt.Println("\n=== Method 2: Line by line ===")
	file, _ := os.Open(testFile)
	defer file.Close()
	
	scanner := bufio.NewScanner(file)
	const maxCapacity = 10 * 1024 * 1024
	buf := make([]byte, maxCapacity)
	scanner.Buffer(buf, maxCapacity)
	
	count2 := 0
	lineNum := 0
	for scanner.Scan() {
		lineNum++
		line := scanner.Text()
		if line == "" {
			continue
		}
		
		// Check if query is in raw line
		if strings.Contains(strings.ToLower(line), strings.ToLower(query)) {
			var msg schemas.SimpleMessage
			if err := json.Unmarshal([]byte(line), &msg); err == nil {
				content := msg.GetContentText()
				if strings.Contains(strings.ToLower(content), strings.ToLower(query)) {
					count2++
					if count2 <= 3 {
						fmt.Printf("Line %d, Type: %s, Content preview: %.50s...\n", lineNum, msg.GetType(), content)
					}
				} else {
					// Query in JSON but not in content
					fmt.Printf("WARNING: Line %d has '%s' in JSON but not in extracted content\n", lineNum, query)
					fmt.Printf("  Type: %s\n", msg.GetType())
					fmt.Printf("  Content: %s\n", content)
				}
			}
		}
	}
	fmt.Printf("Total matches: %d\n", count2)
}