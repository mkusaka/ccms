package search

import (
	"strings"
	"sync"
	"time"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

// SimpleSearchResult contains a matched message
type SimpleSearchResult struct {
	Message   schemas.SimpleMessage
	FilePath  string
	MatchText string
}

// SimpleEngine is the search engine for SimpleMessage
type SimpleEngine struct {
	options SearchOptions
}

// NewSimpleEngine creates a new search engine
func NewSimpleEngine(options SearchOptions) *SimpleEngine {
	return &SimpleEngine{
		options: options,
	}
}

// Search performs a search on the given messages
func (e *SimpleEngine) Search(messages []schemas.SimpleMessage) []SimpleSearchResult {
	var results []SimpleSearchResult
	
	for _, msg := range messages {
		if e.matchesSimpleMessage(msg) {
			result := SimpleSearchResult{
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

// CountMatches counts total number of matching messages without limit
func (e *SimpleEngine) CountMatches(messages []schemas.SimpleMessage) int {
	count := 0
	for _, msg := range messages {
		if e.matchesSimpleMessage(msg) {
			count++
		}
	}
	return count
}

// SearchParallel performs a parallel search on messages
func (e *SimpleEngine) SearchParallel(messages []schemas.SimpleMessage, workers int) []SimpleSearchResult {
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
	resultsChan := make(chan []SimpleSearchResult, workers)
	
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
		go func(chunk []schemas.SimpleMessage) {
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
	var allResults []SimpleSearchResult
	for results := range resultsChan {
		allResults = append(allResults, results...)
		if e.options.MaxResults > 0 && len(allResults) >= e.options.MaxResults {
			allResults = allResults[:e.options.MaxResults]
			break
		}
	}
	
	return allResults
}

// matchesSimpleMessage checks if a message matches the search criteria
func (e *SimpleEngine) matchesSimpleMessage(msg schemas.SimpleMessage) bool {
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