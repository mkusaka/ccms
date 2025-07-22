package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/mkusaka/ccms/golang/internal/search"
)

func main() {
	var (
		pattern    = flag.String("pattern", "", "File pattern to search (e.g., '~/.claude/projects/**/*.jsonl')")
		role       = flag.String("role", "", "Filter by message role (user, assistant, system, summary)")
		sessionID  = flag.String("session", "", "Filter by session ID")
		maxResults = flag.Int("max", 50, "Maximum number of results")
		workers    = flag.Int("workers", runtime.NumCPU(), "Number of parallel workers")
		showHelp   = flag.Bool("help", false, "Show help")
	)

	flag.Parse()

	if *showHelp || flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <query>\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Search Claude session messages for a query string.\n\n")
		fmt.Fprintf(os.Stderr, "Options:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "\nExamples:\n")
		fmt.Fprintf(os.Stderr, "  %s error\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s -role user \"debug\"\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s -pattern \"*.jsonl\" -max 100 \"search term\"\n", os.Args[0])
		os.Exit(0)
	}

	query := strings.Join(flag.Args(), " ")

	// Default pattern if not specified
	searchPattern := *pattern
	if searchPattern == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error getting home directory: %v\n", err)
			os.Exit(1)
		}
		searchPattern = filepath.Join(home, ".claude", "projects", "**", "*.jsonl")
	}

	// Expand home directory
	if strings.HasPrefix(searchPattern, "~") {
		home, err := os.UserHomeDir()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error getting home directory: %v\n", err)
			os.Exit(1)
		}
		searchPattern = filepath.Join(home, searchPattern[1:])
	}

	// Find all matching files
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

	// Create search engine
	engine := search.NewSimpleEngine(search.SearchOptions{
		Query:      query,
		Role:       *role,
		SessionID:  *sessionID,
		MaxResults: *maxResults,
	})

	// Load and search files in parallel
	var allMessages []search.SimpleMessage
	loadResults := search.LoadSimpleMessagesParallel(files, *workers)
	
	totalMessages := 0
	for _, result := range loadResults {
		if result.Error != nil {
			fmt.Fprintf(os.Stderr, "Warning: Failed to load %s: %v\n", result.FilePath, result.Error)
			continue
		}
		totalMessages += len(result.Messages)
		allMessages = append(allMessages, result.Messages...)
	}

	fmt.Fprintf(os.Stderr, "Loaded %d messages, searching...\n", totalMessages)

	// Search with timing
	startTime := time.Now()
	results := engine.SearchParallel(allMessages, *workers)
	
	// Count total matches if we hit the limit
	totalMatches := len(results)
	if *maxResults > 0 && len(results) == *maxResults {
		// Create a counting engine without max results limit
		countEngine := search.NewSimpleEngine(search.SearchOptions{
			Query:      query,
			Role:       *role,
			SessionID:  *sessionID,
			MaxResults: 0, // No limit for counting
		})
		totalMatches = countEngine.CountMatches(allMessages)
	}

	// Calculate search duration
	duration := time.Since(startTime)

	// Display results
	if len(results) == 0 {
		fmt.Println("\nNo results found.")
		fmt.Printf("\n⏱️  Search completed in %dms\n", duration.Milliseconds())
		return
	}

	fmt.Println()

	// Create a file path map for efficient lookup
	filePathMap := make(map[string]string)
	for _, loadResult := range loadResults {
		if loadResult.Error == nil {
			for _, msg := range loadResult.Messages {
				if uuid := msg.GetUUID(); uuid != nil {
					filePathMap[*uuid] = filepath.Base(loadResult.FilePath)
				}
			}
		}
	}

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
		
		// Get file name
		fileName := "unknown"
		if uuid := msg.GetUUID(); uuid != nil {
			if fn, ok := filePathMap[*uuid]; ok {
				fileName = fn
			}
		}
		
		// Get UUID
		uuidStr := ""
		if uuid := msg.GetUUID(); uuid != nil {
			uuidStr = *uuid
		}
		
		// Print header line
		fmt.Printf("%s %s [%s] %s\n", timestampStr, msg.GetType(), fileName, uuidStr)
		
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
	if *maxResults > 0 && len(results) < totalMatches {
		fmt.Printf("(Showing %d of %d total results)\n", len(results), totalMatches)
	}
}