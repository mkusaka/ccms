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

// bytesContainsCI performs case-insensitive search optimized for ASCII
func bytesContainsCI(b, substr []byte) bool {
	if len(substr) == 0 {
		return true
	}
	if len(substr) > len(b) {
		return false
	}
	
	// Use bytes.Contains for case-sensitive search if query is lowercase
	isLowerCase := true
	for _, c := range substr {
		if 'A' <= c && c <= 'Z' {
			isLowerCase = false
			break
		}
	}
	
	if isLowerCase {
		return bytes.Contains(bytes.ToLower(b), substr)
	}
	
	return bytes.Contains(bytes.ToLower(b), bytes.ToLower(substr))
}

// extractContent safely extracts content from a message
func extractContent(line []byte, msgType string) string {
	var content string
	
	switch msgType {
	case "user", "assistant":
		// Try to get message.content
		messageContent, _, _, err := jsonparser.Get(line, "message", "content")
		if err != nil {
			return ""
		}
		
		// Check if it's a string or array
		if len(messageContent) > 0 {
			if messageContent[0] == '"' {
				// Simple string
				content, _ = jsonparser.ParseString(messageContent)
			} else if messageContent[0] == '[' {
				// Array of content items
				var texts []string
				jsonparser.ArrayEach(messageContent, func(value []byte, dataType jsonparser.ValueType, offset int, err error) {
					if err != nil {
						return
					}
					
					itemType, _ := jsonparser.GetString(value, "type")
					switch itemType {
					case "text":
						if text, err := jsonparser.GetString(value, "text"); err == nil {
							texts = append(texts, text)
						}
					case "thinking":
						if text, err := jsonparser.GetString(value, "thinking"); err == nil {
							texts = append(texts, text)
						}
					case "tool_result":
						// Extract tool result content
						toolContent, _, _, err := jsonparser.Get(value, "content")
						if err == nil && len(toolContent) > 0 {
							if toolContent[0] == '"' {
								if text, err := jsonparser.ParseString(toolContent); err == nil {
									texts = append(texts, text)
								}
							} else if toolContent[0] == '[' {
								// Nested array
								jsonparser.ArrayEach(toolContent, func(item []byte, dataType jsonparser.ValueType, offset int, err error) {
									if text, err := jsonparser.GetString(item, "text"); err == nil {
										texts = append(texts, text)
									}
								})
							}
						}
					}
				})
				content = strings.Join(texts, "\n")
			}
		}
		
	case "system":
		content, _ = jsonparser.GetString(line, "content")
		
	case "summary":
		content, _ = jsonparser.GetString(line, "summary")
	}
	
	return content
}

// processFile processes a single file
func processFile(filePath string, query string, results chan<- Result, totalCount *int64, maxResults int, resultCount *int32) {
	data, err := os.ReadFile(filePath)
	if err != nil {
		return
	}

	fileName := filepath.Base(filePath)
	queryLower := strings.ToLower(query)
	lines := bytes.Split(data, []byte{'\n'})
	
	for _, line := range lines {
		if len(line) == 0 {
			continue
		}

		// Fast pre-filter
		if !bytesContainsCI(line, []byte(query)) {
			continue
		}

		// Get message type
		msgType, err := jsonparser.GetString(line, "type")
		if err != nil {
			continue
		}

		// Extract and check content
		content := extractContent(line, msgType)
		if content == "" || !strings.Contains(strings.ToLower(content), queryLower) {
			continue
		}

		// Found a match
		atomic.AddInt64(totalCount, 1)
		
		// Check if we need more results
		currentCount := atomic.LoadInt32(resultCount)
		if maxResults > 0 && int(currentCount) >= maxResults {
			continue
		}

		// Extract metadata
		timestamp, _ := jsonparser.GetString(line, "timestamp")
		uuid, _ := jsonparser.GetString(line, "uuid")
		if uuid == "" && msgType == "summary" {
			uuid, _ = jsonparser.GetString(line, "leafUuid")
		}

		// Send result
		select {
		case results <- Result{
			Timestamp: timestamp,
			Type:      msgType,
			UUID:      uuid,
			Content:   content,
			FileName:  fileName,
		}:
			atomic.AddInt32(resultCount, 1)
		default:
			// Channel full
		}
	}
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

	// Set GOMAXPROCS
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
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}

	if len(files) == 0 {
		fmt.Fprintf(os.Stderr, "No files found\n")
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

			processFile(f, query, results, &totalCount, *maxResults, &resultCount)
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

	// Display each result
	for _, result := range finalResults {
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
		}
	}
	
	// Print footer
	fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
	finalCount := atomic.LoadInt64(&totalCount)
	if *maxResults > 0 && int(finalCount) > len(finalResults) {
		fmt.Printf("(Showing %d of %d total results)\n", len(finalResults), finalCount)
	}
}