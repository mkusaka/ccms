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

// Result holds search result
type Result struct {
	Timestamp string
	Type      string  
	UUID      string
	Content   string
	FileName  string
}

// Global pools for memory reuse
var (
	bufferPool = sync.Pool{
		New: func() interface{} {
			return make([]byte, 0, 1024*1024) // 1MB initial capacity
		},
	}
)

// processFile processes a single file with minimal allocations
func processFile(filePath string, queryLower []byte, results chan<- Result, totalCount *int64, maxResults int) {
	// Read file
	data, err := os.ReadFile(filePath)
	if err != nil {
		return
	}

	fileName := filepath.Base(filePath)
	lines := bytes.Split(data, []byte{'\n'})
	
	for _, line := range lines {
		if len(line) == 0 {
			continue
		}

		// Get type first (fastest check)
		msgType, err := jsonparser.GetString(line, "type")
		if err != nil {
			continue
		}

		// Extract content based on type using jsonparser
		var contentBytes []byte
		var contentStr string
		
		switch msgType {
		case "user", "assistant":
			// Get message.content
			messageContent, dataType, _, err := jsonparser.Get(line, "message", "content")
			if err != nil {
				continue
			}
			
			if dataType == jsonparser.String {
				// Simple string content
				contentStr, _ = jsonparser.ParseString(messageContent)
				contentBytes = []byte(contentStr)
			} else if dataType == jsonparser.Array {
				// Array content - build content incrementally
				buffer := bufferPool.Get().([]byte)[:0]
				
				jsonparser.ArrayEach(messageContent, func(value []byte, dataType jsonparser.ValueType, offset int, err error) {
					itemType, _ := jsonparser.GetString(value, "type")
					switch itemType {
					case "text":
						if text, err := jsonparser.GetString(value, "text"); err == nil {
							if len(buffer) > 0 {
								buffer = append(buffer, '\n')
							}
							buffer = append(buffer, text...)
						}
					case "thinking":
						if text, err := jsonparser.GetString(value, "thinking"); err == nil {
							if len(buffer) > 0 {
								buffer = append(buffer, '\n')
							}
							buffer = append(buffer, text...)
						}
					case "tool_result":
						// Get tool result content
						toolContent, toolType, _, err := jsonparser.Get(value, "content")
						if err == nil {
							if toolType == jsonparser.String {
								if text, err := jsonparser.ParseString(toolContent); err == nil {
									if len(buffer) > 0 {
										buffer = append(buffer, '\n')
									}
									buffer = append(buffer, text...)
								}
							} else if toolType == jsonparser.Array {
								// Handle array of text items
								jsonparser.ArrayEach(toolContent, func(textValue []byte, _ jsonparser.ValueType, _ int, _ error) {
									if text, err := jsonparser.GetString(textValue, "text"); err == nil {
										if len(buffer) > 0 {
											buffer = append(buffer, '\n')
										}
										buffer = append(buffer, text...)
									}
								})
							}
						}
					}
				})
				
				contentBytes = make([]byte, len(buffer))
				copy(contentBytes, buffer)
				bufferPool.Put(buffer)
			}
			
		case "system":
			if content, err := jsonparser.GetString(line, "content"); err == nil {
				contentBytes = []byte(content)
			}
			
		case "summary":
			if content, err := jsonparser.GetString(line, "summary"); err == nil {
				contentBytes = []byte(content)
			}
		}

		// Skip if no content
		if len(contentBytes) == 0 {
			continue
		}

		// Fast case-insensitive search
		if !bytes.Contains(bytes.ToLower(contentBytes), queryLower) {
			continue
		}

		// Found a match
		atomic.AddInt64(totalCount, 1)

		// Extract metadata only if we need to send result
		select {
		case results <- Result{
			Timestamp: func() string {
				ts, _ := jsonparser.GetString(line, "timestamp")
				return ts
			}(),
			Type: msgType,
			UUID: func() string {
				uuid, _ := jsonparser.GetString(line, "uuid")
				if uuid == "" && msgType == "summary" {
					uuid, _ = jsonparser.GetString(line, "leafUuid")
				}
				return uuid
			}(),
			Content:  string(contentBytes),
			FileName: fileName,
		}:
			// Sent
		default:
			// Channel full, continue counting
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
	queryLower := []byte(strings.ToLower(query))

	// Set GOMAXPROCS
	runtime.GOMAXPROCS(runtime.NumCPU())

	// Default pattern
	searchPattern := *pattern
	if searchPattern == "" {
		home, _ := os.UserHomeDir()
		searchPattern = filepath.Join(home, ".claude", "projects", "**", "*.jsonl")
	}

	// Expand home
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

	// Results channel
	results := make(chan Result, *maxResults*2)
	var totalCount int64

	// Process files
	var wg sync.WaitGroup
	sem := make(chan struct{}, *workers)

	for _, file := range files {
		wg.Add(1)
		go func(f string) {
			defer wg.Done()
			
			sem <- struct{}{}
			defer func() { <-sem }()

			processFile(f, queryLower, results, &totalCount, *maxResults)
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

	// Wait
	wg.Wait()
	close(results)
	<-done

	duration := time.Since(startTime)

	// Display
	if len(finalResults) == 0 {
		fmt.Println("\nNo results found.")
		fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
		return
	}

	fmt.Println()

	for _, result := range finalResults {
		// Format timestamp
		timestampStr := result.Timestamp
		if t, err := time.Parse(time.RFC3339, result.Timestamp); err == nil {
			timestampStr = t.Format("2006-01-02 15:04:05")
		}
		
		fmt.Printf("%s %s [%s] %s\n", timestampStr, result.Type, result.FileName, result.UUID)
		
		// Content snippet
		content := result.Content
		pos := strings.Index(strings.ToLower(content), strings.ToLower(query))
		
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
	
	fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
	finalCount := atomic.LoadInt64(&totalCount)
	if *maxResults > 0 && int(finalCount) > len(finalResults) {
		fmt.Printf("(Showing %d of %d total results)\n", len(finalResults), finalCount)
	}
}