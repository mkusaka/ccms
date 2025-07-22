package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"runtime/pprof"
	"strings"
	"time"

	"github.com/mkusaka/ccms/golang/internal/search"
)

func main() {
	var (
		pattern      = flag.String("pattern", "", "File pattern to search")
		role         = flag.String("role", "", "Filter by message role")
		sessionID    = flag.String("session", "", "Filter by session ID")
		maxResults   = flag.Int("max", 50, "Maximum number of results")
		workers      = flag.Int("workers", runtime.NumCPU(), "Number of parallel workers")
		cpuProfile   = flag.String("cpuprofile", "", "Write CPU profile to file")
		showTimings  = flag.Bool("timing", false, "Show detailed timings")
	)

	flag.Parse()

	if flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <query>\n", os.Args[0])
		os.Exit(1)
	}

	// CPU profiling
	if *cpuProfile != "" {
		f, err := os.Create(*cpuProfile)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Could not create CPU profile: %v\n", err)
			os.Exit(1)
		}
		defer f.Close()
		if err := pprof.StartCPUProfile(f); err != nil {
			fmt.Fprintf(os.Stderr, "Could not start CPU profile: %v\n", err)
			os.Exit(1)
		}
		defer pprof.StopCPUProfile()
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

	startTotal := time.Now()

	// Find files
	startGlob := time.Now()
	files, err := filepath.Glob(searchPattern)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error in file pattern: %v\n", err)
		os.Exit(1)
	}
	globDuration := time.Since(startGlob)

	if len(files) == 0 {
		fmt.Fprintf(os.Stderr, "No files found matching pattern: %s\n", searchPattern)
		os.Exit(1)
	}

	if *showTimings {
		fmt.Fprintf(os.Stderr, "Glob pattern match: %v for %d files\n", globDuration, len(files))
	}

	// Load files
	startLoad := time.Now()
	var allMessages []search.SimpleMessage
	loadResults := search.LoadSimpleMessagesParallel(files, *workers)
	
	totalMessages := 0
	for _, result := range loadResults {
		if result.Error != nil {
			continue
		}
		totalMessages += len(result.Messages)
		allMessages = append(allMessages, result.Messages...)
	}
	loadDuration := time.Since(startLoad)

	if *showTimings {
		fmt.Fprintf(os.Stderr, "File loading: %v for %d messages\n", loadDuration, totalMessages)
		fmt.Fprintf(os.Stderr, "Loading speed: %.2f messages/sec\n", float64(totalMessages)/loadDuration.Seconds())
	}

	// Create search engine
	engine := search.NewSimpleEngine(search.SearchOptions{
		Query:      query,
		Role:       *role,
		SessionID:  *sessionID,
		MaxResults: *maxResults,
	})

	// Search
	startSearch := time.Now()
	results := engine.SearchParallel(allMessages, *workers)
	searchDuration := time.Since(startSearch)

	if *showTimings {
		fmt.Fprintf(os.Stderr, "Search execution: %v\n", searchDuration)
		fmt.Fprintf(os.Stderr, "Search speed: %.2f messages/sec\n", float64(totalMessages)/searchDuration.Seconds())
	}

	// Count total matches
	totalMatches := len(results)
	if *maxResults > 0 && len(results) == *maxResults {
		startCount := time.Now()
		countEngine := search.NewSimpleEngine(search.SearchOptions{
			Query:      query,
			Role:       *role,
			SessionID:  *sessionID,
			MaxResults: 0,
		})
		totalMatches = countEngine.CountMatches(allMessages)
		countDuration := time.Since(startCount)
		if *showTimings {
			fmt.Fprintf(os.Stderr, "Count execution: %v\n", countDuration)
		}
	}

	totalDuration := time.Since(startTotal)

	// Display summary
	fmt.Printf("\nFound %d results", totalMatches)
	if *maxResults > 0 && len(results) < totalMatches {
		fmt.Printf(" (showing first %d)", len(results))
	}
	fmt.Printf("\n\n")

	// Display timings
	if *showTimings {
		fmt.Fprintf(os.Stderr, "\n=== Performance Summary ===\n")
		fmt.Fprintf(os.Stderr, "Total time: %v\n", totalDuration)
		fmt.Fprintf(os.Stderr, "- Glob: %v (%.1f%%)\n", globDuration, float64(globDuration)/float64(totalDuration)*100)
		fmt.Fprintf(os.Stderr, "- Load: %v (%.1f%%)\n", loadDuration, float64(loadDuration)/float64(totalDuration)*100)
		fmt.Fprintf(os.Stderr, "- Search: %v (%.1f%%)\n", searchDuration, float64(searchDuration)/float64(totalDuration)*100)
		fmt.Fprintf(os.Stderr, "Files: %d, Messages: %d, Workers: %d\n", len(files), totalMessages, *workers)
	}
}