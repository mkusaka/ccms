package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"time"
)

// FastMessage is a minimal message structure for fast searching
type FastMessage struct {
	Type      string `json:"type"`
	UUID      string `json:"uuid"`
	Timestamp string `json:"timestamp"`
	SessionID string `json:"sessionId"`
	
	// For lazy parsing
	raw []byte
	contentCache *string
}

// GetContent extracts content text lazily
func (m *FastMessage) GetContent() string {
	if m.contentCache != nil {
		return *m.contentCache
	}
	
	content := ""
	
	// Quick extraction based on type
	switch m.Type {
	case "user", "assistant":
		// Find "content" field in raw JSON
		if idx := bytes.Index(m.raw, []byte(`"content":`)); idx >= 0 {
			start := idx + 10
			// Find the actual content start
			for start < len(m.raw) && (m.raw[start] == ' ' || m.raw[start] == '\t') {
				start++
			}
			
			if start < len(m.raw) {
				if m.raw[start] == '"' {
					// Simple string content
					end := start + 1
					for end < len(m.raw) && m.raw[end] != '"' {
						if m.raw[end] == '\\' && end+1 < len(m.raw) {
							end += 2
						} else {
							end++
						}
					}
					if end < len(m.raw) {
						content = string(m.raw[start+1 : end])
					}
				}
			}
		}
	case "system":
		// System messages have content at top level
		if idx := bytes.Index(m.raw, []byte(`"content":`)); idx >= 0 {
			start := idx + 10
			for start < len(m.raw) && (m.raw[start] == ' ' || m.raw[start] == '\t') {
				start++
			}
			if start < len(m.raw) && m.raw[start] == '"' {
				end := start + 1
				for end < len(m.raw) && m.raw[end] != '"' {
					if m.raw[end] == '\\' && end+1 < len(m.raw) {
						end += 2
					} else {
						end++
					}
				}
				if end < len(m.raw) {
					content = string(m.raw[start+1 : end])
				}
			}
		}
	}
	
	m.contentCache = &content
	return content
}

// FastSearchResult holds search results
type FastSearchResult struct {
	Message  FastMessage
	FilePath string
}

// searchFile searches a single file
func searchFile(filePath string, query string, role string, sessionID string, maxResults int) ([]FastSearchResult, int, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, 0, err
	}
	defer file.Close()

	var results []FastSearchResult
	totalMatches := 0
	lowerQuery := strings.ToLower(query)
	
	scanner := bufio.NewScanner(file)
	const maxCapacity = 10 * 1024 * 1024
	buf := make([]byte, maxCapacity)
	scanner.Buffer(buf, maxCapacity)

	fileName := filepath.Base(filePath)

	for scanner.Scan() {
		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}

		// Quick filter: check if query exists in raw JSON
		if query != "" && !bytes.Contains(bytes.ToLower(line), []byte(lowerQuery)) {
			continue
		}

		// Parse minimal fields
		var msg FastMessage
		if err := json.Unmarshal(line, &msg); err != nil {
			continue
		}
		
		// Apply filters
		if role != "" && msg.Type != role {
			continue
		}
		if sessionID != "" && msg.SessionID != sessionID {
			continue
		}

		// Store raw data for lazy parsing
		msg.raw = make([]byte, len(line))
		copy(msg.raw, line)

		// Double-check content if needed
		if query != "" {
			content := msg.GetContent()
			if !strings.Contains(strings.ToLower(content), lowerQuery) {
				continue
			}
		}

		totalMatches++
		
		if maxResults == 0 || len(results) < maxResults {
			results = append(results, FastSearchResult{
				Message:  msg,
				FilePath: fileName,
			})
		}
	}

	return results, totalMatches, scanner.Err()
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

	// Search files in parallel
	var wg sync.WaitGroup
	resultsChan := make(chan []FastSearchResult, len(files))
	countsChan := make(chan int, len(files))
	sem := make(chan struct{}, *workers)

	for _, file := range files {
		wg.Add(1)
		go func(filePath string) {
			defer wg.Done()
			
			sem <- struct{}{}
			defer func() { <-sem }()

			results, count, err := searchFile(filePath, query, *role, *sessionID, *maxResults)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Warning: Failed to search %s: %v\n", filePath, err)
				return
			}
			
			if len(results) > 0 {
				resultsChan <- results
			}
			if count > 0 {
				countsChan <- count
			}
		}(file)
	}

	// Close channels when done
	go func() {
		wg.Wait()
		close(resultsChan)
		close(countsChan)
	}()

	// Collect results
	var allResults []FastSearchResult
	totalMatches := 0
	
	// Collect in parallel
	done := make(chan bool, 2)
	
	go func() {
		for results := range resultsChan {
			allResults = append(allResults, results...)
		}
		done <- true
	}()
	
	go func() {
		for count := range countsChan {
			totalMatches += count
		}
		done <- true
	}()
	
	// Wait for collection to complete
	<-done
	<-done

	duration := time.Since(startTime)

	// Limit results if needed
	if *maxResults > 0 && len(allResults) > *maxResults {
		allResults = allResults[:*maxResults]
	}

	// Display results
	if len(allResults) == 0 {
		fmt.Println("\nNo results found.")
		fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
		return
	}

	fmt.Println()

	// Display each result
	for _, result := range allResults {
		msg := result.Message
		
		// Format timestamp
		timestampStr := ""
		if msg.Timestamp != "" {
			if t, err := time.Parse(time.RFC3339, msg.Timestamp); err == nil {
				timestampStr = t.Format("2006-01-02 15:04:05")
			} else {
				timestampStr = msg.Timestamp
			}
		}
		
		// Print header
		fmt.Printf("%s %s [%s] %s\n", timestampStr, msg.Type, result.FilePath, msg.UUID)
		
		// Show content with context
		content := msg.GetContent()
		if content == "" {
			fmt.Println("  (empty content)")
		} else {
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
	}
	
	// Print footer
	fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
	if *maxResults > 0 && totalMatches > len(allResults) {
		fmt.Printf("(Showing %d of %d total results)\n", len(allResults), totalMatches)
	}
}