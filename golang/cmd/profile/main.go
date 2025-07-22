package main

import (
	"flag"
	"fmt"
	"log"
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
		cpuprofile = flag.String("cpuprofile", "", "write cpu profile to file")
		memprofile = flag.String("memprofile", "", "write memory profile to file")
		detailed   = flag.Bool("detailed", false, "show detailed timing")
	)

	flag.Parse()

	if flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <query>\n", os.Args[0])
		os.Exit(1)
	}

	// Start CPU profiling
	if *cpuprofile != "" {
		f, err := os.Create(*cpuprofile)
		if err != nil {
			log.Fatal(err)
		}
		defer f.Close()
		pprof.StartCPUProfile(f)
		defer pprof.StopCPUProfile()
	}

	query := strings.Join(flag.Args(), " ")

	// Default pattern
	home, _ := os.UserHomeDir()
	searchPattern := filepath.Join(home, ".claude", "projects", "**", "*.jsonl")

	totalStart := time.Now()

	// Phase 1: Find files
	phaseStart := time.Now()
	files, err := filepath.Glob(searchPattern)
	if err != nil {
		log.Fatal(err)
	}
	globTime := time.Since(phaseStart)

	if *detailed {
		fmt.Printf("Phase 1 - File discovery: %v (%d files)\n", globTime, len(files))
	}

	// Phase 2: Load messages
	phaseStart = time.Now()
	var allMessages []search.SimpleMessage
	loadResults := search.LoadSimpleMessagesParallel(files, runtime.NumCPU())
	
	loadedFiles := 0
	for _, result := range loadResults {
		if result.Error == nil {
			loadedFiles++
			allMessages = append(allMessages, result.Messages...)
		}
	}
	loadTime := time.Since(phaseStart)

	if *detailed {
		fmt.Printf("Phase 2 - File loading: %v (%d messages from %d files)\n", 
			loadTime, len(allMessages), loadedFiles)
		fmt.Printf("  Average: %.2f ms/file, %.2f messages/sec\n",
			float64(loadTime.Milliseconds())/float64(loadedFiles),
			float64(len(allMessages))/loadTime.Seconds())
	}

	// Phase 3: Search
	phaseStart = time.Now()
	engine := search.NewSimpleEngine(search.SearchOptions{
		Query:      query,
		MaxResults: 50,
	})
	results := engine.SearchParallel(allMessages, runtime.NumCPU())
	searchTime := time.Since(phaseStart)

	if *detailed {
		fmt.Printf("Phase 3 - Search execution: %v (%d results)\n", searchTime, len(results))
		fmt.Printf("  Speed: %.2f messages/sec\n", float64(len(allMessages))/searchTime.Seconds())
	}

	// Phase 4: Count total (if limited)
	var countTime time.Duration
	totalMatches := len(results)
	if len(results) == 50 {
		phaseStart = time.Now()
		countEngine := search.NewSimpleEngine(search.SearchOptions{
			Query:      query,
			MaxResults: 0,
		})
		totalMatches = countEngine.CountMatches(allMessages)
		countTime = time.Since(phaseStart)
		
		if *detailed {
			fmt.Printf("Phase 4 - Total count: %v (%d total matches)\n", countTime, totalMatches)
		}
	}

	// Phase 5: Format output (simulate)
	phaseStart = time.Now()
	// Create file path map
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
	
	// Format each result
	for _, result := range results {
		msg := result.Message
		timestamp := msg.GetTimestamp()
		if timestamp != nil {
			t, _ := time.Parse(time.RFC3339, *timestamp)
			_ = t.Format("2006-01-02 15:04:05")
		}
		_ = msg.GetContentText()
	}
	formatTime := time.Since(phaseStart)

	if *detailed {
		fmt.Printf("Phase 5 - Output formatting: %v\n", formatTime)
	}

	totalTime := time.Since(totalStart)

	// Summary
	fmt.Printf("\n=== Performance Summary ===\n")
	fmt.Printf("Total time: %v\n", totalTime)
	fmt.Printf("Breakdown:\n")
	fmt.Printf("  File discovery:  %6v (%4.1f%%)\n", globTime, float64(globTime)/float64(totalTime)*100)
	fmt.Printf("  File loading:    %6v (%4.1f%%)\n", loadTime, float64(loadTime)/float64(totalTime)*100)
	fmt.Printf("  Search:          %6v (%4.1f%%)\n", searchTime, float64(searchTime)/float64(totalTime)*100)
	if countTime > 0 {
		fmt.Printf("  Count total:     %6v (%4.1f%%)\n", countTime, float64(countTime)/float64(totalTime)*100)
	}
	fmt.Printf("  Formatting:      %6v (%4.1f%%)\n", formatTime, float64(formatTime)/float64(totalTime)*100)
	
	other := totalTime - globTime - loadTime - searchTime - countTime - formatTime
	if other > 0 {
		fmt.Printf("  Other:           %6v (%4.1f%%)\n", other, float64(other)/float64(totalTime)*100)
	}

	fmt.Printf("\nResults: %d found", totalMatches)
	if len(results) < totalMatches {
		fmt.Printf(" (showing %d)", len(results))
	}
	fmt.Println()

	// Memory profile
	if *memprofile != "" {
		f, err := os.Create(*memprofile)
		if err != nil {
			log.Fatal(err)
		}
		defer f.Close()
		runtime.GC()
		if err := pprof.WriteHeapProfile(f); err != nil {
			log.Fatal(err)
		}
	}
}