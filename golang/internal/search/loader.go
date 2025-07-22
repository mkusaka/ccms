package search

import (
	"bufio"
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"sync"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

// LoadResult contains messages loaded from a file
type LoadResult struct {
	FilePath string
	Messages []schemas.SessionMessage
	Error    error
}

// LoadMessages reads all messages from a JSONL file
func LoadMessages(filePath string) ([]schemas.SessionMessage, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	var messages []schemas.SessionMessage
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

		var msg schemas.SessionMessage
		if err := json.Unmarshal(line, &msg); err != nil {
			// Skip invalid JSON lines (similar to Rust implementation)
			continue
		}

		messages = append(messages, msg)
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	return messages, nil
}

// LoadMessagesParallel loads messages from multiple files in parallel
func LoadMessagesParallel(filePaths []string, workers int) []LoadResult {
	if workers <= 0 {
		workers = 1
	}

	var wg sync.WaitGroup
	results := make([]LoadResult, len(filePaths))
	sem := make(chan struct{}, workers)

	for i, filePath := range filePaths {
		wg.Add(1)
		go func(idx int, path string) {
			defer wg.Done()
			
			sem <- struct{}{}        // Acquire
			defer func() { <-sem }() // Release

			messages, err := LoadMessages(path)
			results[idx] = LoadResult{
				FilePath: path,
				Messages: messages,
				Error:    err,
			}
		}(i, filePath)
	}

	wg.Wait()
	return results
}

// LoadMessagesFromPattern loads messages from files matching a glob pattern
func LoadMessagesFromPattern(pattern string) ([]schemas.SessionMessage, error) {
	files, err := filepath.Glob(pattern)
	if err != nil {
		return nil, err
	}

	var allMessages []schemas.SessionMessage
	for _, file := range files {
		messages, err := LoadMessages(file)
		if err != nil {
			// Skip files that can't be read
			continue
		}
		allMessages = append(allMessages, messages...)
	}

	return allMessages, nil
}

// StreamMessages reads messages from a reader one at a time
func StreamMessages(reader io.Reader, handler func(schemas.SessionMessage) error) error {
	scanner := bufio.NewScanner(reader)
	
	// Increase buffer size for large lines
	const maxCapacity = 10 * 1024 * 1024 // 10MB
	buf := make([]byte, maxCapacity)
	scanner.Buffer(buf, maxCapacity)

	for scanner.Scan() {
		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}

		var msg schemas.SessionMessage
		if err := json.Unmarshal(line, &msg); err != nil {
			// Skip invalid JSON lines
			continue
		}

		if err := handler(msg); err != nil {
			return err
		}
	}

	return scanner.Err()
}