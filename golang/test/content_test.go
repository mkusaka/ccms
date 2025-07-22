package test

import (
	"encoding/json"
	"strings"
	"testing"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

func TestContentExtraction(t *testing.T) {
	tests := []struct {
		name     string
		jsonData string
		expected string
		contains string
	}{
		{
			name: "user message with simple string content",
			jsonData: `{
				"type": "user",
				"message": {"role": "user", "content": "Hello error world"},
				"uuid": "test1",
				"timestamp": "2024-01-01T00:00:00Z",
				"sessionId": "s1"
			}`,
			expected: "Hello error world",
			contains: "error",
		},
		{
			name: "user message with array content",
			jsonData: `{
				"type": "user",
				"message": {
					"role": "user",
					"content": [
						{"type": "text", "text": "First error text"},
						{"type": "text", "text": "Second text"}
					]
				},
				"uuid": "test2",
				"timestamp": "2024-01-01T00:00:00Z",
				"sessionId": "s1"
			}`,
			expected: "First error text\nSecond text",
			contains: "error",
		},
		{
			name: "assistant message with tool use",
			jsonData: `{
				"type": "assistant",
				"message": {
					"id": "msg1",
					"type": "message",
					"role": "assistant",
					"model": "claude",
					"content": [
						{"type": "text", "text": "Let me fix this error"},
						{"type": "tool_use", "id": "tool1", "name": "bash", "input": {}}
					],
					"usage": {"input_tokens": 100, "output_tokens": 50}
				},
				"uuid": "test3",
				"timestamp": "2024-01-01T00:00:00Z",
				"sessionId": "s1"
			}`,
			expected: "Let me fix this error",
			contains: "error",
		},
		{
			name: "system message",
			jsonData: `{
				"type": "system",
				"content": "System error occurred",
				"uuid": "test4",
				"timestamp": "2024-01-01T00:00:00Z",
				"sessionId": "s1",
				"isMeta": false
			}`,
			expected: "System error occurred",
			contains: "error",
		},
		{
			name: "assistant with thinking",
			jsonData: `{
				"type": "assistant",
				"message": {
					"id": "msg2",
					"type": "message",
					"role": "assistant",
					"model": "claude",
					"content": [
						{"type": "thinking", "thinking": "This is an error case", "signature": "sig"},
						{"type": "text", "text": "Here's the answer"}
					],
					"usage": {"input_tokens": 100, "output_tokens": 50}
				},
				"uuid": "test5",
				"timestamp": "2024-01-01T00:00:00Z",
				"sessionId": "s1"
			}`,
			expected: "This is an error case\nHere's the answer",
			contains: "error",
		},
		{
			name: "user with tool result",
			jsonData: `{
				"type": "user",
				"message": {
					"role": "user",
					"content": [
						{"type": "tool_result", "tool_use_id": "tool1", "content": "Error: file not found"}
					]
				},
				"uuid": "test6",
				"timestamp": "2024-01-01T00:00:00Z",
				"sessionId": "s1"
			}`,
			expected: "Error: file not found",
			contains: "error",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var msg schemas.SimpleMessage
			err := json.Unmarshal([]byte(tt.jsonData), &msg)
			if err != nil {
				t.Fatalf("Failed to unmarshal: %v", err)
			}

			content := msg.GetContentText()
			
			// Check if content matches expected
			if content != tt.expected {
				t.Errorf("Content mismatch:\nGot:      %q\nExpected: %q", content, tt.expected)
			}

			// Check if contains search term
			if !strings.Contains(strings.ToLower(content), strings.ToLower(tt.contains)) {
				t.Errorf("Content %q should contain %q", content, tt.contains)
			}
		})
	}
}