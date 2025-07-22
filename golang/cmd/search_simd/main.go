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

// processFile processes a single file using simdjson
func processFile(filePath string, queryLower string, results chan<- Result, totalCount *int64, maxResults int) {
	data, err := os.ReadFile(filePath)
	if err != nil {
		return
	}

	fileName := filepath.Base(filePath)
	lines := bytes.Split(data, []byte{'\n'})
	
	// Reuse parser and parsed object
	reusableParser := &simdjson.Parser{}
	pj := &simdjson.ParsedJson{}
	
	for _, line := range lines {
		if len(line) == 0 {
			continue
		}

		// Parse JSON line
		if err := pj.ParseBytes(line, reusableParser); err != nil {
			continue
		}
		
		iter := &simdjson.Iter{}
		iter.AdvanceInto(pj)
		
		// Navigate to root object
		if t := iter.Type(); t != simdjson.TypeObject {
			continue
		}
		
		// Extract fields manually by iterating
		var msgType, content, timestamp, uuid string
		
		// Move into object
		iter.AdvanceIntoObject()
		
		for iter.Type() != simdjson.TypeNone {
			key, err := iter.ReadString()
			if err != nil {
				iter.Skip()
				continue
			}
			
			switch key {
			case "type":
				msgType, _ = iter.ReadString()
			case "timestamp":
				timestamp, _ = iter.ReadString()
			case "uuid":
				uuid, _ = iter.ReadString()
			case "leafUuid":
				if msgType == "summary" && uuid == "" {
					uuid, _ = iter.ReadString()
				} else {
					iter.Skip()
				}
			case "content":
				if msgType == "system" {
					content, _ = iter.ReadString()
				} else {
					iter.Skip()
				}
			case "summary":
				if msgType == "summary" {
					content, _ = iter.ReadString()
				} else {
					iter.Skip()
				}
			case "message":
				if msgType == "user" || msgType == "assistant" {
					// Handle message object
					if iter.Type() == simdjson.TypeObject {
						iter.AdvanceIntoObject()
						
						for iter.Type() != simdjson.TypeNone {
							msgKey, err := iter.ReadString()
							if err != nil {
								iter.Skip()
								continue
							}
							
							if msgKey == "content" {
								// Handle content which can be string or array
								if iter.Type() == simdjson.TypeString {
									content, _ = iter.ReadString()
								} else if iter.Type() == simdjson.TypeArray {
									// Extract text from array
									var texts []string
									iter.AdvanceIntoArray()
									
									for iter.Type() != simdjson.TypeNone {
										if iter.Type() == simdjson.TypeObject {
											iter.AdvanceIntoObject()
											
											var itemType, text string
											for iter.Type() != simdjson.TypeNone {
												itemKey, _ := iter.ReadString()
												switch itemKey {
												case "type":
													itemType, _ = iter.ReadString()
												case "text":
													if itemType == "text" {
														text, _ = iter.ReadString()
													} else {
														iter.Skip()
													}
												case "thinking":
													if itemType == "thinking" {
														text, _ = iter.ReadString()
													} else {
														iter.Skip()
													}
												case "content":
													if itemType == "tool_result" {
														if iter.Type() == simdjson.TypeString {
															text, _ = iter.ReadString()
														} else {
															iter.Skip()
														}
													} else {
														iter.Skip()
													}
												default:
													iter.Skip()
												}
											}
											
											if text != "" {
												texts = append(texts, text)
											}
										} else {
											iter.Skip()
										}
									}
									
									content = strings.Join(texts, "\n")
								} else {
									iter.Skip()
								}
							} else {
								iter.Skip()
							}
						}
					} else {
						iter.Skip()
					}
				} else {
					iter.Skip()
				}
			default:
				iter.Skip()
			}
		}

		// Check if content matches
		if content == "" || !strings.Contains(strings.ToLower(content), queryLower) {
			continue
		}

		// Found a match
		atomic.AddInt64(totalCount, 1)

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