package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"time"

	"github.com/mkusaka/ccms/golang/internal/schemas"
	"github.com/mkusaka/ccms/golang/internal/search"
)

// createTestData creates a temporary JSONL file with test data
func createTestData(numLines int) (string, error) {
	tempDir, err := ioutil.TempDir("", "ccms-benchmark-*")
	if err != nil {
		return "", err
	}

	testFile := filepath.Join(tempDir, "test.jsonl")
	file, err := os.Create(testFile)
	if err != nil {
		return "", err
	}
	defer file.Close()

	for i := 0; i < numLines; i++ {
		msg := map[string]interface{}{
			"type": "user",
			"message": map[string]interface{}{
				"role":    "user",
				"content": fmt.Sprintf("Message %d with some test content for searching", i),
			},
			"uuid":        fmt.Sprintf("uuid-%d", i),
			"timestamp":   fmt.Sprintf("2024-01-01T00:00:%02dZ", i%60),
			"sessionId":   "session1",
			"parentUuid":  nil,
			"isSidechain": false,
			"userType":    "external",
			"cwd":         "/test",
			"version":     "1.0",
		}

		data, err := json.Marshal(msg)
		if err != nil {
			return "", err
		}

		if _, err := file.Write(data); err != nil {
			return "", err
		}
		if _, err := file.WriteString("\n"); err != nil {
			return "", err
		}
	}

	return testFile, nil
}

// benchmarkSearch runs a search benchmark
func benchmarkSearch(name string, numLines int, query string, workers int) {
	fmt.Printf("\n=== Benchmark: %s ===\n", name)
	fmt.Printf("Lines: %d, Query: %q, Workers: %d\n", numLines, query, workers)

	// Create test data
	testFile, err := createTestData(numLines)
	if err != nil {
		log.Fatalf("Failed to create test data: %v", err)
	}
	defer os.RemoveAll(filepath.Dir(testFile))

	// Warm up
	messages, err := search.LoadMessages(testFile)
	if err != nil {
		log.Fatalf("Failed to load messages: %v", err)
	}
	fmt.Printf("Loaded %d messages\n", len(messages))

	// Create search engine
	engine := search.NewEngine(search.SearchOptions{
		Query:      query,
		MaxResults: 0, // No limit
	})

	// Run benchmark
	const iterations = 10
	var totalDuration time.Duration

	for i := 0; i < iterations; i++ {
		start := time.Now()
		results := engine.SearchParallel(messages, workers)
		duration := time.Since(start)
		totalDuration += duration

		if i == 0 {
			fmt.Printf("Found %d results\n", len(results))
		}
	}

	avgDuration := totalDuration / iterations
	fmt.Printf("Average time: %v\n", avgDuration)
	fmt.Printf("Throughput: %.2f messages/sec\n", float64(numLines)/avgDuration.Seconds())
}

// benchmarkJSONParsing benchmarks JSON parsing speed
func benchmarkJSONParsing(numLines int) {
	fmt.Printf("\n=== Benchmark: JSON Parsing ===\n")
	fmt.Printf("Lines: %d\n", numLines)

	// Create a sample JSON line
	msg := map[string]interface{}{
		"type": "user",
		"message": map[string]interface{}{
			"role":    "user",
			"content": "Test message with some content for parsing benchmark",
		},
		"uuid":        "test-uuid",
		"timestamp":   "2024-01-01T00:00:00Z",
		"sessionId":   "session1",
		"parentUuid":  nil,
		"isSidechain": false,
		"userType":    "external",
		"cwd":         "/test",
		"version":     "1.0",
	}

	data, err := json.Marshal(msg)
	if err != nil {
		log.Fatalf("Failed to marshal test message: %v", err)
	}

	const iterations = 100000
	start := time.Now()

	for i := 0; i < iterations; i++ {
		var parsed schemas.SessionMessage
		if err := json.Unmarshal(data, &parsed); err != nil {
			log.Fatalf("Failed to unmarshal: %v", err)
		}
	}

	duration := time.Since(start)
	fmt.Printf("Parsed %d messages in %v\n", iterations, duration)
	fmt.Printf("Throughput: %.2f messages/sec\n", float64(iterations)/duration.Seconds())
	fmt.Printf("Per message: %v\n", duration/iterations)
}

// benchmarkFileLoading benchmarks file loading speed
func benchmarkFileLoading(numLines int) {
	fmt.Printf("\n=== Benchmark: File Loading ===\n")
	fmt.Printf("Lines: %d\n", numLines)

	// Create test data
	testFile, err := createTestData(numLines)
	if err != nil {
		log.Fatalf("Failed to create test data: %v", err)
	}
	defer os.RemoveAll(filepath.Dir(testFile))

	const iterations = 5
	var totalDuration time.Duration

	for i := 0; i < iterations; i++ {
		start := time.Now()
		messages, err := search.LoadMessages(testFile)
		duration := time.Since(start)

		if err != nil {
			log.Fatalf("Failed to load messages: %v", err)
		}

		totalDuration += duration

		if i == 0 {
			fmt.Printf("Loaded %d messages\n", len(messages))
		}
	}

	avgDuration := totalDuration / iterations
	fmt.Printf("Average load time: %v\n", avgDuration)
	fmt.Printf("Throughput: %.2f messages/sec\n", float64(numLines)/avgDuration.Seconds())
}

func main() {
	fmt.Println("Claude Code Message Search - Go Benchmark")
	fmt.Printf("Go version: %s\n", runtime.Version())
	fmt.Printf("NumCPU: %d\n", runtime.NumCPU())
	fmt.Printf("GOMAXPROCS: %d\n", runtime.GOMAXPROCS(0))

	// JSON parsing benchmark
	benchmarkJSONParsing(10000)

	// File loading benchmarks
	benchmarkFileLoading(1000)
	benchmarkFileLoading(10000)
	benchmarkFileLoading(100000)

	// Search benchmarks with different worker counts
	workers := []int{1, 2, 4, 8, runtime.NumCPU()}

	// Simple search
	for _, w := range workers {
		benchmarkSearch("Simple Search (1K)", 1000, "test", w)
	}

	// Larger dataset
	for _, w := range workers {
		benchmarkSearch("Simple Search (10K)", 10000, "test", w)
	}

	// Even larger dataset
	for _, w := range workers {
		benchmarkSearch("Simple Search (100K)", 100000, "test", w)
	}

	// Complex query patterns
	benchmarkSearch("Specific Content Search (10K)", 10000, "Message 500", runtime.NumCPU())
	benchmarkSearch("Case Insensitive Search (10K)", 10000, "TEST", runtime.NumCPU())

	fmt.Println("\nBenchmark complete!")
}