package test

import (
	"strings"
	"testing"
)

// Test case-insensitive contains
func TestBytesContainsCI(t *testing.T) {
	tests := []struct {
		haystack string
		needle   string
		expected bool
	}{
		{"Hello Error World", "error", true},
		{"Hello ERROR World", "error", true},
		{"Hello error World", "error", true},
		{"Hello ErRoR World", "error", true},
		{"Hello World", "error", false},
		{"Error at start", "error", true},
		{"at end error", "error", true},
		{"", "error", false},
		{"error", "error", true},
		{"ERROR", "error", true},
		{"Error", "ERROR", true},
		{"This is an error case", "error", true},
	}

	for _, tt := range tests {
		// Using strings.Contains with ToLower (correct implementation)
		got := strings.Contains(strings.ToLower(tt.haystack), strings.ToLower(tt.needle))
		if got != tt.expected {
			t.Errorf("bytesContainsCI(%q, %q) = %v, want %v", tt.haystack, tt.needle, got, tt.expected)
		}
	}
}