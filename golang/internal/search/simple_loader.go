package search

import (
	"bufio"
	"encoding/json"
	"os"
	"sync"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

// LoadSimpleMessages reads all messages from a JSONL file using SimpleMessage
func LoadSimpleMessages(filePath string) ([]schemas.SimpleMessage, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	var messages []schemas.SimpleMessage
	scanner := bufio.NewScanner(file)
	
	// Increase buffer size for large lines
	const maxCapacity = 10 * 1024 * 1024 // 10MB
	buf := make([]byte, maxCapacity)
	scanner.Buffer(buf, maxCapacity)

	lineNum := 0
	for scanner.Scan() {
		lineNum++
		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}

		var msg schemas.SimpleMessage
		if err := json.Unmarshal(line, &msg); err != nil {
			// Skip invalid JSON lines
			continue
		}

		messages = append(messages, msg)
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	return messages, nil
}

// SimpleMessage is an alias for external use
type SimpleMessage = schemas.SimpleMessage

// SimpleLoadResult contains messages loaded from a file
type SimpleLoadResult struct {
	FilePath string
	Messages []schemas.SimpleMessage
	Error    error
}

// LoadSimpleMessagesParallel loads messages from multiple files in parallel
func LoadSimpleMessagesParallel(filePaths []string, workers int) []SimpleLoadResult {
	if workers <= 0 {
		workers = 1
	}

	var wg sync.WaitGroup
	results := make([]SimpleLoadResult, len(filePaths))
	sem := make(chan struct{}, workers)

	for i, filePath := range filePaths {
		wg.Add(1)
		go func(idx int, path string) {
			defer wg.Done()
			
			sem <- struct{}{}        // Acquire
			defer func() { <-sem }() // Release

			messages, err := LoadSimpleMessages(path)
			results[idx] = SimpleLoadResult{
				FilePath: path,
				Messages: messages,
				Error:    err,
			}
		}(i, filePath)
	}

	wg.Wait()
	return results
}