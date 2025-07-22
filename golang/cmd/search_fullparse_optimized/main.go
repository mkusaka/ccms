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

// Message represents a complete session message with all fields
type Message struct {
	Type      string
	Timestamp string
	UUID      string
	SessionID string // Not a pointer anymore
	Content   string // Extracted content text for search
	
	// Message data - using concrete types instead of interface{}
	MessageID   string
	MessageRole string
	ContentType byte // 0: none, 1: string, 2: array
	
	// For different message types
	SystemContent  string
	SummaryContent string
	LeafUUID       string
	
	// Content items for array content
	ContentItems []ContentItem
}

// ContentItem represents items in content array
type ContentItem struct {
	Type         string
	Text         string
	Thinking     string
	ToolContent  string
	IsToolArray  bool
}

// Result holds search result with full message
type Result struct {
	Message  Message
	FileName string
}

// Global pools for memory reuse
var (
	bufferPool = sync.Pool{
		New: func() interface{} {
			return make([]byte, 0, 1024*1024) // 1MB initial capacity
		},
	}
	
	messagePool = sync.Pool{
		New: func() interface{} {
			return &Message{
				ContentItems: make([]ContentItem, 0, 10), // Pre-allocate for common case
			}
		},
	}
)

// processFile processes a single file with early filtering
func processFile(filePath string, queryLower []byte, roleFilter string, sessionFilter string, results chan<- Result, totalCount *int64, maxResults int) {
	// Read file
	data, err := os.ReadFile(filePath)
	if err != nil {
		return
	}

	fileName := filepath.Base(filePath)
	lines := bytes.Split(data, []byte{'\n'})
	
	// Pre-compile paths
	typePath := []string{"type"}
	
	for _, line := range lines {
		if len(line) == 0 {
			continue
		}

		// Early type check for role filtering
		msgType, err := jsonparser.GetString(line, typePath...)
		if err != nil {
			continue
		}
		
		// Early role filter
		if roleFilter != "" && msgType != roleFilter {
			continue
		}
		
		// Early session filter if specified
		if sessionFilter != "" {
			if sessionID, err := jsonparser.GetString(line, "sessionId"); err != nil || sessionID != sessionFilter {
				continue
			}
		}

		// Get message from pool and reset
		msg := messagePool.Get().(*Message)
		msg.Type = msgType
		msg.Timestamp = ""
		msg.UUID = ""
		msg.SessionID = ""
		msg.MessageID = ""
		msg.MessageRole = ""
		msg.ContentType = 0
		msg.SystemContent = ""
		msg.SummaryContent = ""
		msg.LeafUUID = ""
		msg.ContentItems = msg.ContentItems[:0] // Reset slice but keep capacity
		
		// Get buffer for content extraction
		buffer := bufferPool.Get().([]byte)[:0]
		
		// Extract content based on type
		switch msgType {
		case "user", "assistant":
			// Get message object
			messageData, _, _, err := jsonparser.Get(line, "message")
			if err == nil {
				// Get message content
				contentData, dataType, _, err := jsonparser.Get(messageData, "content")
				if err == nil {
					if dataType == jsonparser.String {
						// Simple string content
						content, _ := jsonparser.ParseString(contentData)
						msg.ContentType = 1
						buffer = append(buffer, content...)
					} else if dataType == jsonparser.Array {
						// Array content
						msg.ContentType = 2
						
						jsonparser.ArrayEach(contentData, func(value []byte, dataType jsonparser.ValueType, offset int, err error) {
							item := ContentItem{}
							
							// Get item type
							if itemType, err := jsonparser.GetString(value, "type"); err == nil {
								item.Type = itemType
							}
							
							switch item.Type {
							case "text":
								if text, err := jsonparser.GetString(value, "text"); err == nil {
									item.Text = text
									if len(buffer) > 0 {
										buffer = append(buffer, '\n')
									}
									buffer = append(buffer, text...)
								}
							case "thinking":
								if thinking, err := jsonparser.GetString(value, "thinking"); err == nil {
									item.Thinking = thinking
									if len(buffer) > 0 {
										buffer = append(buffer, '\n')
									}
									buffer = append(buffer, thinking...)
								}
							case "tool_result":
								// Get tool result content
								toolContent, toolType, _, err := jsonparser.Get(value, "content")
								if err == nil {
									if toolType == jsonparser.String {
										if text, err := jsonparser.ParseString(toolContent); err == nil {
											item.ToolContent = text
											if len(buffer) > 0 {
												buffer = append(buffer, '\n')
											}
											buffer = append(buffer, text...)
										}
									} else if toolType == jsonparser.Array {
										item.IsToolArray = true
										// Extract text from array
										jsonparser.ArrayEach(toolContent, func(textValue []byte, _ jsonparser.ValueType, _ int, _ error) {
											if text, err := jsonparser.GetString(textValue, "text"); err == nil {
												if len(buffer) > 0 {
													buffer = append(buffer, '\n')
												}
												buffer = append(buffer, text...)
												if item.ToolContent != "" {
													item.ToolContent += "\n"
												}
												item.ToolContent += text
											}
										})
									}
								}
							}
							
							if item.Type != "" {
								msg.ContentItems = append(msg.ContentItems, item)
							}
						})
					}
				}
			}
			
		case "system":
			if content, err := jsonparser.GetString(line, "content"); err == nil {
				msg.SystemContent = content
				buffer = append(buffer, content...)
			}
			
		case "summary":
			if summary, err := jsonparser.GetString(line, "summary"); err == nil {
				msg.SummaryContent = summary
				buffer = append(buffer, summary...)
			}
		}
		
		// Skip if no content
		if len(buffer) == 0 {
			bufferPool.Put(buffer)
			messagePool.Put(msg)
			continue
		}

		// Fast case-insensitive search
		if !bytes.Contains(bytes.ToLower(buffer), queryLower) {
			bufferPool.Put(buffer)
			messagePool.Put(msg)
			continue
		}

		// Found a match - now extract remaining metadata
		atomic.AddInt64(totalCount, 1)
		
		// Store content
		msg.Content = string(buffer)
		bufferPool.Put(buffer)
		
		// Lazy extraction of metadata only for matches
		if timestamp, err := jsonparser.GetString(line, "timestamp"); err == nil {
			msg.Timestamp = timestamp
		}
		
		if uuid, err := jsonparser.GetString(line, "uuid"); err == nil {
			msg.UUID = uuid
		} else if msgType == "summary" {
			// Try leafUuid for summary
			if leafUuid, err := jsonparser.GetString(line, "leafUuid"); err == nil {
				msg.LeafUUID = leafUuid
				msg.UUID = leafUuid
			}
		}
		
		if sessionID, err := jsonparser.GetString(line, "sessionId"); err == nil {
			msg.SessionID = sessionID
		}
		
		// Extract message-specific metadata for user/assistant
		if msgType == "user" || msgType == "assistant" {
			if messageData, _, _, err := jsonparser.Get(line, "message"); err == nil {
				if id, err := jsonparser.GetString(messageData, "id"); err == nil {
					msg.MessageID = id
				}
				if role, err := jsonparser.GetString(messageData, "role"); err == nil {
					msg.MessageRole = role
				}
			}
		}

		// Send result
		select {
		case results <- Result{
			Message:  *msg, // Copy the message
			FileName: fileName,
		}:
			// Sent
		default:
			// Channel full, continue counting
		}
		
		// Return message to pool
		messagePool.Put(msg)
	}
}

func main() {
	var (
		pattern    = flag.String("pattern", "", "File pattern to search")
		maxResults = flag.Int("max", 50, "Maximum number of results")
		workers    = flag.Int("workers", runtime.NumCPU(), "Number of parallel workers")
		role       = flag.String("role", "", "Filter by message role")
		sessionID  = flag.String("session", "", "Filter by session ID")
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

			processFile(f, queryLower, *role, *sessionID, results, &totalCount, *maxResults)
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
		timestampStr := result.Message.Timestamp
		if t, err := time.Parse(time.RFC3339, result.Message.Timestamp); err == nil {
			timestampStr = t.Format("2006-01-02 15:04:05")
		}
		
		fmt.Printf("%s %s [%s] %s\n", timestampStr, result.Message.Type, result.FileName, result.Message.UUID)
		
		// Content snippet
		content := result.Message.Content
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
	
	// デバッグ用：最初の結果の完全な構造を表示
	if false && len(finalResults) > 0 {
		fmt.Printf("\n=== First result full structure ===\n")
		r := finalResults[0]
		fmt.Printf("Type: %s\n", r.Message.Type)
		fmt.Printf("UUID: %s\n", r.Message.UUID)
		fmt.Printf("SessionID: %s\n", r.Message.SessionID)
		fmt.Printf("MessageID: %s\n", r.Message.MessageID)
		fmt.Printf("MessageRole: %s\n", r.Message.MessageRole)
		fmt.Printf("ContentType: %d\n", r.Message.ContentType)
		fmt.Printf("ContentItems: %d items\n", len(r.Message.ContentItems))
		for i, item := range r.Message.ContentItems {
			fmt.Printf("  [%d] Type: %s\n", i, item.Type)
			if item.Text != "" {
				fmt.Printf("      Text: %d chars\n", len(item.Text))
			}
			if item.Thinking != "" {
				fmt.Printf("      Thinking: %d chars\n", len(item.Thinking))
			}
			if item.ToolContent != "" {
				fmt.Printf("      ToolContent: %d chars (IsArray: %v)\n", len(item.ToolContent), item.IsToolArray)
			}
		}
	}
}