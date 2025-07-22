package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mkusaka/ccms/golang/internal/search"
)

func main() {
	// デフォルトパターンで検索
	home, _ := os.UserHomeDir()
	pattern := filepath.Join(home, ".claude", "projects", "**", "*.jsonl")
	
	files, err := filepath.Glob(pattern)
	if err != nil {
		panic(err)
	}
	
	fmt.Printf("Found %d files\n", len(files))
	
	// 各ファイルの内容を確認
	totalMessages := 0
	errorMatches := 0
	
	for _, file := range files {
		messages, err := search.LoadSimpleMessages(file)
		if err != nil {
			fmt.Printf("Error loading %s: %v\n", file, err)
			continue
		}
		
		fileMatches := 0
		for _, msg := range messages {
			totalMessages++
			content := msg.GetContentText()
			if strings.Contains(strings.ToLower(content), "error") {
				errorMatches++
				fileMatches++
			}
		}
		
		if fileMatches > 0 {
			fmt.Printf("%s: %d messages, %d matches\n", filepath.Base(file), len(messages), fileMatches)
		}
	}
	
	fmt.Printf("\nTotal messages: %d\n", totalMessages)
	fmt.Printf("Messages containing 'error': %d\n", errorMatches)
	
	// Rust風の検索も試す
	fmt.Println("\n--- Testing different content extraction ---")
	
	// サンプルメッセージで確認
	testFile := files[0]
	messages, _ := search.LoadSimpleMessages(testFile)
	if len(messages) > 0 {
		for i := 0; i < 5 && i < len(messages); i++ {
			msg := messages[i]
			content := msg.GetContentText()
			fmt.Printf("Message %d (type=%s): content length=%d\n", i, msg.GetType(), len(content))
			if len(content) > 100 {
				fmt.Printf("  Preview: %s...\n", content[:100])
			} else {
				fmt.Printf("  Content: %s\n", content)
			}
		}
	}
}