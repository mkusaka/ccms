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

	"github.com/minio/simdjson-go"
)

// Result holds search result
type Result struct {
	Timestamp string
	Type      string
	UUID      string
	Content   string
	FileName  string
}

// extractContent extracts content from different message types
func extractContent(obj *simdjson.Object, msgType string) string {
	switch msgType {
	case "user", "assistant":
		// Get message.content
		if msgElem := obj.FindKey("message", nil); msgElem != nil {
			msgObj, err := msgElem.Iter.Object(nil)
			if err != nil {
				return ""
			}
			
			if contentElem := msgObj.FindKey("content", nil); contentElem != nil {
				// Check if it's a string or array
				contentIter := contentElem.Iter
				typ := contentIter.Type()
				
				if typ == simdjson.TypeString {
					content, _ := contentIter.String()
					return content
				} else if typ == simdjson.TypeArray {
					// Handle array content
					var texts []string
					arr, err := contentIter.Array(nil)
					if err != nil {
						return ""
					}
					
					arr.ForEach(func(i simdjson.Iter) {
						itemObj, err := i.Object(nil)
						if err != nil {
							return
						}
						
						// Get item type
						var itemType string
						if typeElem := itemObj.FindKey("type", nil); typeElem != nil {
							itemType, _ = typeElem.Iter.String()
						}
						
						switch itemType {
						case "text":
							if textElem := itemObj.FindKey("text", nil); textElem != nil {
								if text, err := textElem.Iter.String(); err == nil {
									texts = append(texts, text)
								}
							}
						case "thinking":
							if thinkingElem := itemObj.FindKey("thinking", nil); thinkingElem != nil {
								if text, err := thinkingElem.Iter.String(); err == nil {
									texts = append(texts, text)
								}
							}
						case "tool_result":
							if contentElem := itemObj.FindKey("content", nil); contentElem != nil {
								if contentElem.Iter.Type() == simdjson.TypeString {
									if text, err := contentElem.Iter.String(); err == nil {
										texts = append(texts, text)
									}
								} else if contentElem.Iter.Type() == simdjson.TypeArray {
									// Handle array of text items
									if arr, err := contentElem.Iter.Array(nil); err == nil {
										arr.ForEach(func(j simdjson.Iter) {
											if textObj, err := j.Object(nil); err == nil {
												if textElem := textObj.FindKey("text", nil); textElem != nil {
													if text, err := textElem.Iter.String(); err == nil {
														texts = append(texts, text)
													}
												}
											}
										})
									}
								}
							}
						}
					})
					
					return strings.Join(texts, "\n")
				}
			}
		}
		
	case "system":
		if contentElem := obj.FindKey("content", nil); contentElem != nil {
			content, _ := contentElem.Iter.String()
			return content
		}
		
	case "summary":
		if summaryElem := obj.FindKey("summary", nil); summaryElem != nil {
			content, _ := summaryElem.Iter.String()
			return content
		}
	}
	
	return ""
}

// processFile processes a single file using simdjson
func processFile(filePath string, queryLower string, results chan<- Result, totalCount *int64, maxResults int) {
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

		// Parse JSON line
		pj, err := simdjson.Parse(line, nil)
		if err != nil {
			continue
		}
		
		// Get iterator and advance to root
		iter := pj.Iter()
		if iter.Advance() != simdjson.TypeObject {
			continue
		}
		
		// Convert to object
		obj, err := iter.Object(nil)
		if err != nil {
			continue
		}
		
		// Get message type
		var msgType string
		if typeElem := obj.FindKey("type", nil); typeElem != nil {
			msgType, _ = typeElem.Iter.String()
		}
		
		if msgType == "" {
			continue
		}
		
		// Extract content
		content := extractContent(obj, msgType)
		if content == "" || !strings.Contains(strings.ToLower(content), queryLower) {
			continue
		}

		// Found a match
		atomic.AddInt64(totalCount, 1)

		// Extract other fields
		var timestamp, uuid string
		
		if tsElem := obj.FindKey("timestamp", nil); tsElem != nil {
			timestamp, _ = tsElem.Iter.String()
		}
		
		if uuidElem := obj.FindKey("uuid", nil); uuidElem != nil {
			uuid, _ = uuidElem.Iter.String()
		}
		
		// For summary messages, check leafUuid if uuid is empty
		if uuid == "" && msgType == "summary" {
			if leafElem := obj.FindKey("leafUuid", nil); leafElem != nil {
				uuid, _ = leafElem.Iter.String()
			}
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
	queryLower := strings.ToLower(query)

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
		pos := strings.Index(strings.ToLower(content), queryLower)
		
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