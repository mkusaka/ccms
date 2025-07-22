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
	SessionID *string
	Content   string // Extracted content text for search
	
	// Original message data
	Message   *MessageContent `json:"message,omitempty"`
	System    *string        `json:"content,omitempty"`    // For system messages
	Summary   *string        `json:"summary,omitempty"`    // For summary messages
	LeafUUID  *string        `json:"leafUuid,omitempty"`   // For summary messages
}

// MessageContent represents the message field for user/assistant messages
type MessageContent struct {
	ID      string      `json:"id"`
	Content interface{} `json:"content"` // Can be string or []ContentItem
	Role    string      `json:"role"`
}

// ContentItem represents items in content array
type ContentItem struct {
	Type     string      `json:"type"`
	Text     *string     `json:"text,omitempty"`
	Thinking *string     `json:"thinking,omitempty"`
	Content  interface{} `json:"content,omitempty"` // For tool_result
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
)

// parseMessage parses a complete message from JSON line
func parseMessage(line []byte) (*Message, error) {
	msg := &Message{}
	
	// Get type
	msgType, err := jsonparser.GetString(line, "type")
	if err != nil {
		return nil, err
	}
	msg.Type = msgType
	
	// Get common fields
	if timestamp, err := jsonparser.GetString(line, "timestamp"); err == nil {
		msg.Timestamp = timestamp
	}
	
	if uuid, err := jsonparser.GetString(line, "uuid"); err == nil {
		msg.UUID = uuid
	}
	
	if sessionID, err := jsonparser.GetString(line, "sessionId"); err == nil {
		msg.SessionID = &sessionID
	}
	
	// Parse type-specific fields and extract content
	buffer := bufferPool.Get().([]byte)[:0]
	
	switch msgType {
	case "user", "assistant":
		// Get message object
		messageData, _, _, err := jsonparser.Get(line, "message")
		if err == nil {
			msgContent := &MessageContent{}
			
			// Get message ID
			if id, err := jsonparser.GetString(messageData, "id"); err == nil {
				msgContent.ID = id
			}
			
			// Get role
			if role, err := jsonparser.GetString(messageData, "role"); err == nil {
				msgContent.Role = role
			}
			
			// Get content
			contentData, dataType, _, err := jsonparser.Get(messageData, "content")
			if err == nil {
				if dataType == jsonparser.String {
					// Simple string content
					content, _ := jsonparser.ParseString(contentData)
					msgContent.Content = content
					buffer = append(buffer, content...)
				} else if dataType == jsonparser.Array {
					// Array content
					var items []ContentItem
					
					jsonparser.ArrayEach(contentData, func(value []byte, dataType jsonparser.ValueType, offset int, err error) {
						item := ContentItem{}
						
						// Get item type
						if itemType, err := jsonparser.GetString(value, "type"); err == nil {
							item.Type = itemType
						}
						
						switch item.Type {
						case "text":
							if text, err := jsonparser.GetString(value, "text"); err == nil {
								item.Text = &text
								if len(buffer) > 0 {
									buffer = append(buffer, '\n')
								}
								buffer = append(buffer, text...)
							}
						case "thinking":
							if thinking, err := jsonparser.GetString(value, "thinking"); err == nil {
								item.Thinking = &thinking
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
										item.Content = text
										if len(buffer) > 0 {
											buffer = append(buffer, '\n')
										}
										buffer = append(buffer, text...)
									}
								} else if toolType == jsonparser.Array {
									// Handle array of text items
									var textItems []map[string]string
									jsonparser.ArrayEach(toolContent, func(textValue []byte, _ jsonparser.ValueType, _ int, _ error) {
										textItem := make(map[string]string)
										if text, err := jsonparser.GetString(textValue, "text"); err == nil {
											textItem["text"] = text
											if len(buffer) > 0 {
												buffer = append(buffer, '\n')
											}
											buffer = append(buffer, text...)
										}
										textItems = append(textItems, textItem)
									})
									item.Content = textItems
								}
							}
						}
						
						items = append(items, item)
					})
					
					msgContent.Content = items
				}
			}
			
			msg.Message = msgContent
		}
		
	case "system":
		if content, err := jsonparser.GetString(line, "content"); err == nil {
			msg.System = &content
			buffer = append(buffer, content...)
		}
		
	case "summary":
		if summary, err := jsonparser.GetString(line, "summary"); err == nil {
			msg.Summary = &summary
			buffer = append(buffer, summary...)
		}
		// Get leafUuid if uuid is empty
		if msg.UUID == "" {
			if leafUuid, err := jsonparser.GetString(line, "leafUuid"); err == nil {
				msg.LeafUUID = &leafUuid
				msg.UUID = leafUuid // Use leafUuid as UUID for summary
			}
		}
	}
	
	// Store extracted content
	msg.Content = string(buffer)
	bufferPool.Put(buffer)
	
	return msg, nil
}

// processFile processes a single file
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

		// Parse complete message
		msg, err := parseMessage(line)
		if err != nil {
			continue
		}

		// Skip if no content
		if len(msg.Content) == 0 {
			continue
		}

		// Fast case-insensitive search
		if !bytes.Contains([]byte(strings.ToLower(msg.Content)), queryLower) {
			continue
		}

		// Found a match
		atomic.AddInt64(totalCount, 1)

		// Send result
		select {
		case results <- Result{
			Message:  *msg,
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

			processFile(f, queryLower, results, &totalCount, *maxResults)
		}(file)
	}

	// Collect results with filtering
	var finalResults []Result
	done := make(chan bool)
	
	go func() {
		for result := range results {
			// Apply filters
			if *role != "" && result.Message.Type != *role {
				continue
			}
			if *sessionID != "" && (result.Message.SessionID == nil || *result.Message.SessionID != *sessionID) {
				continue
			}
			
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
	
	// デバッグ用：最初の結果の完全な構造を表示（コメントアウト可能）
	/*
	if len(finalResults) > 0 {
		fmt.Printf("\n=== First result full structure ===\n")
		r := finalResults[0]
		fmt.Printf("Type: %s\n", r.Message.Type)
		fmt.Printf("UUID: %s\n", r.Message.UUID)
		if r.Message.SessionID != nil {
			fmt.Printf("SessionID: %s\n", *r.Message.SessionID)
		}
		if r.Message.Message != nil {
			fmt.Printf("Message.ID: %s\n", r.Message.Message.ID)
			fmt.Printf("Message.Role: %s\n", r.Message.Message.Role)
		}
	}
	*/
}