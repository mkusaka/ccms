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

	"github.com/bytedance/sonic"
	"github.com/mkusaka/ccms/golang/internal/schemas"
)

// SearchResult holds a result with file info
type SearchResult struct {
	Message  schemas.SimpleMessage
	FileName string
}

// searchFileStream searches a file using sonic for JSON parsing
func searchFileStream(filePath string, query string, role string, sessionID string, maxResults int, resultsChan chan<- SearchResult, totalCount *int64, wg *sync.WaitGroup) {
	defer wg.Done()

	data, err := os.ReadFile(filePath)
	if err != nil {
		return
	}

	lowerQuery := strings.ToLower(query)
	fileName := filepath.Base(filePath)
	localCount := 0
	lines := bytes.Split(data, []byte{'\n'})

	for _, line := range lines {
		if len(line) == 0 {
			continue
		}

		// Use sonic for fast JSON parsing
		var msg schemas.SimpleMessage
		if err := sonic.Unmarshal(line, &msg); err != nil {
			continue
		}

		// Apply filters
		if role != "" && msg.GetType() != role {
			continue
		}
		if sessionID != "" {
			sid := msg.GetSessionID()
			if sid == nil || *sid != sessionID {
				continue
			}
		}

		// Check content
		content := msg.GetContentText()
		if query != "" && !strings.Contains(strings.ToLower(content), lowerQuery) {
			continue
		}

		// Found a match
		localCount++
		
		// Send result if within limit
		select {
		case resultsChan <- SearchResult{Message: msg, FileName: fileName}:
			// Sent successfully
		default:
			// Channel full, just count
		}
	}

	// Update total count
	if localCount > 0 {
		atomic.AddInt64(totalCount, int64(localCount))
	}
}

func main() {
	var (
		pattern    = flag.String("pattern", "", "File pattern to search")
		role       = flag.String("role", "", "Filter by message role")
		sessionID  = flag.String("session", "", "Filter by session ID")
		maxResults = flag.Int("max", 50, "Maximum number of results")
		workers    = flag.Int("workers", runtime.NumCPU(), "Number of parallel workers")
	)

	flag.Parse()

	if flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <query>\n", os.Args[0])
		os.Exit(1)
	}

	query := strings.Join(flag.Args(), " ")

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

	// Create results channel with buffer
	resultsChan := make(chan SearchResult, *maxResults)
	var totalCount int64
	var wg sync.WaitGroup

	// Worker pool
	sem := make(chan struct{}, *workers)

	// Process files
	for _, file := range files {
		wg.Add(1)
		go func(filePath string) {
			sem <- struct{}{}        // Acquire
			defer func() { <-sem }() // Release

			searchFileStream(filePath, query, *role, *sessionID, *maxResults, resultsChan, &totalCount, &wg)
		}(file)
	}

	// Collect results
	var results []SearchResult
	done := make(chan bool)
	
	go func() {
		for result := range resultsChan {
			results = append(results, result)
			if *maxResults > 0 && len(results) >= *maxResults {
				break
			}
		}
		done <- true
	}()

	// Wait for all workers
	wg.Wait()
	close(resultsChan)
	<-done

	duration := time.Since(startTime)

	// Get final count
	finalCount := atomic.LoadInt64(&totalCount)

	// Display results
	if len(results) == 0 {
		fmt.Println("\nNo results found.")
		fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
		return
	}

	fmt.Println()

	// Create a file path map for efficient lookup
	filePathMap := make(map[string]string)
	for _, r := range results {
		if uuid := r.Message.GetUUID(); uuid != nil {
			filePathMap[*uuid] = r.FileName
		}
	}

	// Display each result
	for _, result := range results {
		msg := result.Message
		
		// Format timestamp
		timestampStr := ""
		if timestamp := msg.GetTimestamp(); timestamp != nil {
			if t, err := time.Parse(time.RFC3339, *timestamp); err == nil {
				timestampStr = t.Format("2006-01-02 15:04:05")
			} else {
				timestampStr = *timestamp
			}
		}
		
		// Get UUID
		uuidStr := ""
		if uuid := msg.GetUUID(); uuid != nil {
			uuidStr = *uuid
		}
		
		// Print header line
		fmt.Printf("%s %s [%s] %s\n", timestampStr, msg.GetType(), result.FileName, uuidStr)
		
		// Show content with context
		content := msg.GetContentText()
		if content == "" {
			fmt.Println("  (empty content)")
		} else {
			// Find query position and show context
			lowerContent := strings.ToLower(content)
			lowerQuery := strings.ToLower(query)
			pos := strings.Index(lowerContent, lowerQuery)
			
			if pos >= 0 {
				// Show context around the match
				contextSize := 50
				start := pos - contextSize
				if start < 0 {
					start = 0
				}
				end := pos + len(query) + contextSize
				if end > len(content) {
					end = len(content)
				}
				
				// Clean up the content (remove newlines for display)
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
				// No match found in content, show beginning
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
	}
	
	// Print footer
	fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
	if *maxResults > 0 && int(finalCount) > len(results) {
		fmt.Printf("(Showing %d of %d total results)\n", len(results), finalCount)
	}
}