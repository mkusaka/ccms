package main

import (
	"bytes"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/buger/jsonparser"
)

// Result holds a search result
type Result struct {
	Timestamp string
	Type      string
	UUID      string
	Content   string
	FileName  string
}

// processChunk processes a chunk of file data
func processChunk(data []byte, fileName string, queryBytes []byte, results chan<- Result, totalCount *int64, maxResults int, resultCount *int32) {
	lines := bytes.Split(data, []byte{'\n'})
	
	for _, line := range lines {
		if len(line) == 0 {
			continue
		}

		// Fast pre-filter using custom case-insensitive search
		if !bytesContainsCI(line, queryBytes) {
			continue
		}

		// Extract only needed fields using jsonparser
		msgType, _ := jsonparser.GetString(line, "type")
		if msgType == "" {
			continue
		}

		// Extract content based on type
		var content []byte
		var err error
		
		switch msgType {
		case "user", "assistant":
			// Try to get content from message.content
			content, _, _, err = jsonparser.Get(line, "message", "content")
			if err == nil && len(content) > 0 {
				// Handle array content
				if content[0] == '[' {
					var allTexts []string
					jsonparser.ArrayEach(content, func(value []byte, dataType jsonparser.ValueType, offset int, err error) {
						itemType, _ := jsonparser.GetString(value, "type")
						switch itemType {
						case "text":
							if text, err := jsonparser.GetString(value, "text"); err == nil {
								allTexts = append(allTexts, text)
							}
						case "thinking":
							if text, err := jsonparser.GetString(value, "thinking"); err == nil {
								allTexts = append(allTexts, text)
							}
						case "tool_result":
							if toolContent, _, _, err := jsonparser.Get(value, "content"); err == nil {
								if toolContent[0] == '"' {
									if text, err := jsonparser.ParseString(toolContent); err == nil {
										allTexts = append(allTexts, text)
									}
								}
							}
						}
					})
					content = []byte(strings.Join(allTexts, "\n"))
				} else if content[0] == '"' {
					// Simple string content
					if text, err := jsonparser.ParseString(content); err == nil {
						content = []byte(text)
					}
				}
			}
		case "system":
			if text, err := jsonparser.GetString(line, "content"); err == nil {
				content = []byte(text)
			}
		case "summary":
			if text, err := jsonparser.GetString(line, "summary"); err == nil {
				content = []byte(text)
			}
		}

		// Check content match
		if len(content) == 0 || !bytesContainsCI(content, queryBytes) {
			continue
		}

		// Found a match
		atomic.AddInt64(totalCount, 1)
		
		// Only process if we need more results
		currentCount := atomic.LoadInt32(resultCount)
		if maxResults > 0 && int(currentCount) >= maxResults {
			continue
		}

		// Extract other fields
		timestamp, _ := jsonparser.GetString(line, "timestamp")
		uuid, _ := jsonparser.GetString(line, "uuid")
		if uuid == "" {
			uuid, _ = jsonparser.GetString(line, "leafUuid")
		}

		// Try to send result
		select {
		case results <- Result{
			Timestamp: timestamp,
			Type:      msgType,
			UUID:      uuid,
			Content:   string(content),
			FileName:  fileName,
		}:
			atomic.AddInt32(resultCount, 1)
		default:
			// Channel full
		}
	}
}

// processFile processes a single file
func processFile(filePath string, queryBytes []byte, results chan<- Result, totalCount *int64, maxResults int, resultCount *int32) error {
	data, err := os.ReadFile(filePath)
	if err != nil {
		return err
	}

	fileName := filepath.Base(filePath)
	processChunk(data, fileName, queryBytes, results, totalCount, maxResults, resultCount)
	return nil
}

// bytesContainsCI performs case-insensitive search optimized for ASCII
func bytesContainsCI(b, substr []byte) bool {
	if len(substr) == 0 {
		return true
	}
	if len(substr) > len(b) {
		return false
	}
	
	// Convert pattern to lowercase once
	pattern := make([]byte, len(substr))
	for i, c := range substr {
		if 'A' <= c && c <= 'Z' {
			pattern[i] = c + 32
		} else {
			pattern[i] = c
		}
	}
	
	firstChar := pattern[0]
	for i := 0; i <= len(b)-len(pattern); i++ {
		// Fast check for first character
		c := b[i]
		if 'A' <= c && c <= 'Z' {
			c = c + 32
		}
		if c != firstChar {
			continue
		}
		
		// Check remaining characters
		match := true
		for j := 1; j < len(pattern); j++ {
			c2 := b[i+j]
			if 'A' <= c2 && c2 <= 'Z' {
				c2 = c2 + 32
			}
			if c2 != pattern[j] {
				match = false
				break
			}
		}
		if match {
			return true
		}
	}
	return false
}

func main() {
	var (
		pattern    = flag.String("pattern", "", "File pattern to search")
		maxResults = flag.Int("max", 50, "Maximum number of results")
		workers    = flag.Int("workers", runtime.NumCPU(), "Number of parallel workers")
	)

	flag.Parse()

	if flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <query>\n", os.Args[0])
		os.Exit(1)
	}

	query := strings.Join(flag.Args(), " ")
	queryBytes := []byte(query)

	// Set GOMAXPROCS explicitly
	runtime.GOMAXPROCS(runtime.NumCPU())

	// Default pattern
	searchPattern := *pattern
	if searchPattern == "" {
		home, _ := os.UserHomeDir()
		searchPattern = filepath.Join(home, ".claude", "projects", "**", "*.jsonl")
	}

	// Expand home directory
	if strings.HasPrefix(searchPattern, "~") {
		home, _ := os.UserHomeDir()
		searchPattern = filepath.Join(home, searchPattern[1:])
	}

	// Find files
	files, err := filepath.Glob(searchPattern)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error in file pattern: %v\n", err)
		os.Exit(1)
	}

	if len(files) == 0 {
		fmt.Fprintf(os.Stderr, "No files found matching pattern: %s\n", searchPattern)
		os.Exit(1)
	}

	fmt.Fprintf(os.Stderr, "Searching %d files for: %q\n", len(files), query)

	startTime := time.Now()

	// Results channel and counters
	results := make(chan Result, *maxResults*2)
	var totalCount int64
	var resultCount int32

	// Process files in parallel
	var wg sync.WaitGroup
	sem := make(chan struct{}, *workers)

	for _, file := range files {
		wg.Add(1)
		go func(f string) {
			defer wg.Done()
			
			sem <- struct{}{}
			defer func() { <-sem }()

			processFile(f, queryBytes, results, &totalCount, *maxResults, &resultCount)
		}(file)
	}

	// Collect results
	var finalResults []Result
	done := make(chan bool)
	
	go func() {
		for result := range results {
			finalResults = append(finalResults, result)
			if *maxResults > 0 && len(finalResults) >= *maxResults {
				break
			}
		}
		done <- true
	}()

	// Wait for processing
	wg.Wait()
	close(results)
	<-done

	duration := time.Since(startTime)

	// Display results
	if len(finalResults) == 0 {
		fmt.Println("\nNo results found.")
		fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
		return
	}

	fmt.Println()

	// Format and display each result
	for _, result := range finalResults {
		// Format timestamp
		timestampStr := result.Timestamp
		if t, err := time.Parse(time.RFC3339, result.Timestamp); err == nil {
			timestampStr = t.Format("2006-01-02 15:04:05")
		}
		
		fmt.Printf("%s %s [%s] %s\n", timestampStr, result.Type, result.FileName, result.UUID)
		
		// Show content snippet
		content := result.Content
		lowerContent := strings.ToLower(content)
		lowerQuery := strings.ToLower(query)
		pos := strings.Index(lowerContent, lowerQuery)
		
		if pos >= 0 {
			contextSize := 50
			start := pos - contextSize
			if start < 0 {
				start = 0
			}
			end := pos + len(query) + contextSize
			if end > len(content) {
				end = len(content)
			}
			
			snippet := strings.ReplaceAll(content[start:end], "\n", " ")
			snippet = strings.ReplaceAll(snippet, "\t", " ")
			
			prefix := ""
			if start > 0 {
				prefix = "..."
			}
			suffix := ""
			if end < len(content) {
				suffix = "..."
			}
			
			fmt.Printf("  %s%s%s\n", prefix, snippet, suffix)
		} else {
			maxLen := 150
			if len(content) > maxLen {
				snippet := strings.ReplaceAll(content[:maxLen], "\n", " ")
				snippet = strings.ReplaceAll(snippet, "\t", " ")
				fmt.Printf("  %s...\n", snippet)
			} else {
				snippet := strings.ReplaceAll(content, "\n", " ")
				snippet = strings.ReplaceAll(snippet, "\t", " ")
				fmt.Printf("  %s\n", snippet)
			}
		}
	}
	
	// Print footer
	fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
	finalCount := atomic.LoadInt64(&totalCount)
	if *maxResults > 0 && int(finalCount) > len(finalResults) {
		fmt.Printf("(Showing %d of %d total results)\n", len(finalResults), finalCount)
	}
}