package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"runtime/pprof"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

type Metrics struct {
	filesProcessed   int64
	linesRead        int64
	linesPreFiltered int64
	jsonParsed       int64
	contentExtracted int64
	matchesFound     int64
	
	readTime      int64 // nanoseconds
	preFilterTime int64
	parseTime     int64
	extractTime   int64
	matchTime     int64
	totalTime     int64
}

func (m *Metrics) Add(other *Metrics) {
	atomic.AddInt64(&m.filesProcessed, other.filesProcessed)
	atomic.AddInt64(&m.linesRead, other.linesRead)
	atomic.AddInt64(&m.linesPreFiltered, other.linesPreFiltered)
	atomic.AddInt64(&m.jsonParsed, other.jsonParsed)
	atomic.AddInt64(&m.contentExtracted, other.contentExtracted)
	atomic.AddInt64(&m.matchesFound, other.matchesFound)
	atomic.AddInt64(&m.readTime, other.readTime)
	atomic.AddInt64(&m.preFilterTime, other.preFilterTime)
	atomic.AddInt64(&m.parseTime, other.parseTime)
	atomic.AddInt64(&m.extractTime, other.extractTime)
	atomic.AddInt64(&m.matchTime, other.matchTime)
	atomic.AddInt64(&m.totalTime, other.totalTime)
}

func searchFileWithMetrics(filePath string, query string) (*Metrics, int) {
	fileStart := time.Now()
	metrics := &Metrics{filesProcessed: 1}
	matches := 0

	file, err := os.Open(filePath)
	if err != nil {
		return metrics, 0
	}
	defer file.Close()

	lowerQuery := strings.ToLower(query)
	
	scanner := bufio.NewScanner(file)
	const maxCapacity = 10 * 1024 * 1024
	buf := make([]byte, maxCapacity)
	scanner.Buffer(buf, maxCapacity)

	for scanner.Scan() {
		// Phase 1: Read line
		start := time.Now()
		line := scanner.Bytes()
		metrics.linesRead++
		metrics.readTime += time.Since(start).Nanoseconds()
		
		if len(line) == 0 {
			continue
		}

		// Phase 2: Pre-filter
		start = time.Now()
		if !strings.Contains(strings.ToLower(string(line)), lowerQuery) {
			metrics.preFilterTime += time.Since(start).Nanoseconds()
			continue
		}
		metrics.linesPreFiltered++
		metrics.preFilterTime += time.Since(start).Nanoseconds()

		// Phase 3: JSON parse
		start = time.Now()
		var msg schemas.SimpleMessage
		if err := json.Unmarshal(line, &msg); err != nil {
			metrics.parseTime += time.Since(start).Nanoseconds()
			continue
		}
		metrics.jsonParsed++
		metrics.parseTime += time.Since(start).Nanoseconds()

		// Phase 4: Extract content
		start = time.Now()
		content := msg.GetContentText()
		metrics.contentExtracted++
		metrics.extractTime += time.Since(start).Nanoseconds()

		// Phase 5: Match content
		start = time.Now()
		if strings.Contains(strings.ToLower(content), lowerQuery) {
			matches++
			metrics.matchesFound++
		}
		metrics.matchTime += time.Since(start).Nanoseconds()
	}

	metrics.totalTime = time.Since(fileStart).Nanoseconds()
	return metrics, matches
}

func main() {
	var (
		cpuprofile = flag.String("cpuprofile", "", "write cpu profile to file")
		workers    = flag.Int("workers", runtime.NumCPU(), "number of workers")
	)

	flag.Parse()

	if flag.NArg() == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <query>\n", os.Args[0])
		os.Exit(1)
	}

	// CPU profiling
	if *cpuprofile != "" {
		f, err := os.Create(*cpuprofile)
		if err != nil {
			panic(err)
		}
		defer f.Close()
		pprof.StartCPUProfile(f)
		defer pprof.StopCPUProfile()
	}

	query := strings.Join(flag.Args(), " ")

	// Find files
	home, _ := os.UserHomeDir()
	pattern := filepath.Join(home, ".claude", "projects", "**", "*.jsonl")
	
	startTime := time.Now()
	files, err := filepath.Glob(pattern)
	if err != nil {
		panic(err)
	}
	globTime := time.Since(startTime)

	fmt.Printf("Found %d files in %v\n", len(files), globTime)

	// Process files
	startTime = time.Now()
	
	var globalMetrics Metrics
	var totalMatches int64
	var wg sync.WaitGroup
	sem := make(chan struct{}, *workers)

	for _, file := range files {
		wg.Add(1)
		go func(f string) {
			defer wg.Done()
			
			sem <- struct{}{}
			defer func() { <-sem }()

			metrics, matches := searchFileWithMetrics(f, query)
			globalMetrics.Add(metrics)
			atomic.AddInt64(&totalMatches, int64(matches))
		}(file)
	}

	wg.Wait()
	processTime := time.Since(startTime)

	// Print results
	fmt.Printf("\n=== Search Performance Analysis ===\n")
	fmt.Printf("Query: %q\n", query)
	fmt.Printf("Workers: %d\n", *workers)
	fmt.Printf("\n")

	fmt.Printf("Overall:\n")
	fmt.Printf("  Total time: %v\n", processTime)
	fmt.Printf("  Files processed: %d\n", globalMetrics.filesProcessed)
	fmt.Printf("  Matches found: %d\n", totalMatches)
	fmt.Printf("\n")

	fmt.Printf("Processing stats:\n")
	fmt.Printf("  Lines read: %d\n", globalMetrics.linesRead)
	fmt.Printf("  Lines pre-filtered: %d (%.1f%% of read)\n", 
		globalMetrics.linesPreFiltered, 
		float64(globalMetrics.linesPreFiltered)/float64(globalMetrics.linesRead)*100)
	fmt.Printf("  JSON parsed: %d (%.1f%% of pre-filtered)\n", 
		globalMetrics.jsonParsed,
		float64(globalMetrics.jsonParsed)/float64(globalMetrics.linesPreFiltered)*100)
	fmt.Printf("  Content extracted: %d\n", globalMetrics.contentExtracted)
	fmt.Printf("  Matches found: %d (%.1f%% of parsed)\n", 
		globalMetrics.matchesFound,
		float64(globalMetrics.matchesFound)/float64(globalMetrics.jsonParsed)*100)
	fmt.Printf("\n")

	// Calculate average times
	avgReadTime := time.Duration(globalMetrics.readTime / globalMetrics.linesRead)
	avgPreFilterTime := time.Duration(globalMetrics.preFilterTime / globalMetrics.linesRead)
	avgParseTime := time.Duration(0)
	if globalMetrics.linesPreFiltered > 0 {
		avgParseTime = time.Duration(globalMetrics.parseTime / globalMetrics.linesPreFiltered)
	}
	avgExtractTime := time.Duration(0)
	avgMatchTime := time.Duration(0)
	if globalMetrics.jsonParsed > 0 {
		avgExtractTime = time.Duration(globalMetrics.extractTime / globalMetrics.jsonParsed)
		avgMatchTime = time.Duration(globalMetrics.matchTime / globalMetrics.jsonParsed)
	}

	fmt.Printf("Average time per operation:\n")
	fmt.Printf("  Read line: %v\n", avgReadTime)
	fmt.Printf("  Pre-filter: %v\n", avgPreFilterTime)
	fmt.Printf("  JSON parse: %v (per pre-filtered line)\n", avgParseTime)
	fmt.Printf("  Extract content: %v (per parsed message)\n", avgExtractTime)
	fmt.Printf("  Match check: %v (per parsed message)\n", avgMatchTime)
	fmt.Printf("\n")

	// Estimate total time spent in each phase
	totalCPUTime := globalMetrics.readTime + globalMetrics.preFilterTime + 
		globalMetrics.parseTime + globalMetrics.extractTime + globalMetrics.matchTime
	
	fmt.Printf("CPU time breakdown:\n")
	fmt.Printf("  Reading: %v (%.1f%%)\n", 
		time.Duration(globalMetrics.readTime), 
		float64(globalMetrics.readTime)/float64(totalCPUTime)*100)
	fmt.Printf("  Pre-filtering: %v (%.1f%%)\n", 
		time.Duration(globalMetrics.preFilterTime),
		float64(globalMetrics.preFilterTime)/float64(totalCPUTime)*100)
	fmt.Printf("  JSON parsing: %v (%.1f%%)\n", 
		time.Duration(globalMetrics.parseTime),
		float64(globalMetrics.parseTime)/float64(totalCPUTime)*100)
	fmt.Printf("  Content extraction: %v (%.1f%%)\n", 
		time.Duration(globalMetrics.extractTime),
		float64(globalMetrics.extractTime)/float64(totalCPUTime)*100)
	fmt.Printf("  Matching: %v (%.1f%%)\n", 
		time.Duration(globalMetrics.matchTime),
		float64(globalMetrics.matchTime)/float64(totalCPUTime)*100)
	fmt.Printf("  Total CPU time: %v\n", time.Duration(totalCPUTime))
	fmt.Printf("  Total file processing time: %v\n", time.Duration(globalMetrics.totalTime))
	fmt.Printf("\n")

	// Performance metrics
	fmt.Printf("Performance:\n")
	fmt.Printf("  Lines/sec: %.0f\n", float64(globalMetrics.linesRead)/processTime.Seconds())
	fmt.Printf("  JSON parses/sec: %.0f\n", float64(globalMetrics.jsonParsed)/processTime.Seconds())
	fmt.Printf("  Wall time efficiency: %.1f%% (CPU time / wall time)\n",
		float64(totalCPUTime)/float64(processTime*time.Duration(*workers))*100)
}