package search

import (
	"strings"
	"sync"
	"time"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

// SearchOptions contains options for searching
type SearchOptions struct {
	Query       string
	Role        string
	SessionID   string
	MaxResults  int
	Before      *time.Time
	After       *time.Time
	FullText    bool
}

// SearchResult contains a matched message
type SearchResult struct {
	Message   schemas.SessionMessage
	FilePath  string
	MatchText string
}

// Engine is the search engine
type Engine struct {
	options SearchOptions
}

// NewEngine creates a new search engine
func NewEngine(options SearchOptions) *Engine {
	return &Engine{
		options: options,
	}
}

// Search performs a search on the given messages
func (e *Engine) Search(messages []schemas.SessionMessage) []SearchResult {
	var results []SearchResult
	
	for _, msg := range messages {
		if e.matchesMessage(msg) {
			result := SearchResult{
				Message:   msg,
				MatchText: msg.GetContentText(),
			}
			results = append(results, result)
			
			if e.options.MaxResults > 0 && len(results) >= e.options.MaxResults {
				break
			}
		}
	}
	
	return results
}

// SearchParallel performs a parallel search on messages
func (e *Engine) SearchParallel(messages []schemas.SessionMessage, workers int) []SearchResult {
	if workers <= 0 {
		workers = 1
	}
	
	// For small datasets, use single-threaded search
	if len(messages) < 1000 {
		return e.Search(messages)
	}
	
	// Divide messages into chunks
	chunkSize := (len(messages) + workers - 1) / workers
	var wg sync.WaitGroup
	resultsChan := make(chan []SearchResult, workers)
	
	for i := 0; i < workers; i++ {
		start := i * chunkSize
		end := start + chunkSize
		if end > len(messages) {
			end = len(messages)
		}
		
		if start >= end {
			break
		}
		
		wg.Add(1)
		go func(chunk []schemas.SessionMessage) {
			defer wg.Done()
			results := e.Search(chunk)
			resultsChan <- results
		}(messages[start:end])
	}
	
	// Wait for all workers to complete
	go func() {
		wg.Wait()
		close(resultsChan)
	}()
	
	// Collect results
	var allResults []SearchResult
	for results := range resultsChan {
		allResults = append(allResults, results...)
		if e.options.MaxResults > 0 && len(allResults) >= e.options.MaxResults {
			allResults = allResults[:e.options.MaxResults]
			break
		}
	}
	
	return allResults
}

// matchesMessage checks if a message matches the search criteria
func (e *Engine) matchesMessage(msg schemas.SessionMessage) bool {
	// Filter by role
	if e.options.Role != "" && msg.GetType() != e.options.Role {
		return false
	}
	
	// Filter by session ID
	if e.options.SessionID != "" {
		sessionID := msg.GetSessionID()
		if sessionID == nil || *sessionID != e.options.SessionID {
			return false
		}
	}
	
	// Filter by timestamp
	if e.options.Before != nil || e.options.After != nil {
		timestamp := msg.GetTimestamp()
		if timestamp == nil {
			return false
		}
		
		msgTime, err := time.Parse(time.RFC3339, *timestamp)
		if err != nil {
			return false
		}
		
		if e.options.Before != nil && msgTime.After(*e.options.Before) {
			return false
		}
		
		if e.options.After != nil && msgTime.Before(*e.options.After) {
			return false
		}
	}
	
	// Filter by query
	if e.options.Query != "" {
		content := msg.GetContentText()
		if !strings.Contains(strings.ToLower(content), strings.ToLower(e.options.Query)) {
			return false
		}
	}
	
	return true
}

// SearchFiles searches multiple files for messages
func (e *Engine) SearchFiles(filePaths []string, workers int) ([]SearchResult, error) {
	loadResults := LoadMessagesParallel(filePaths, workers)
	
	var allMessages []schemas.SessionMessage
	for _, result := range loadResults {
		if result.Error != nil {
			// Skip files with errors
			continue
		}
		
		// Add file path info to results
		for _, msg := range result.Messages {
			allMessages = append(allMessages, msg)
		}
	}
	
	return e.SearchParallel(allMessages, workers), nil
}

// SearchPattern searches files matching a pattern
func (e *Engine) SearchPattern(pattern string, workers int) ([]SearchResult, error) {
	messages, err := LoadMessagesFromPattern(pattern)
	if err != nil {
		return nil, err
	}
	
	return e.SearchParallel(messages, workers), nil
}