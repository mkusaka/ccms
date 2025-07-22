package test

import (
	"encoding/json"
	"fmt"
	"testing"

	"github.com/mkusaka/ccms/golang/internal/schemas"
)

func TestMessageParsing(t *testing.T) {
	// Test data that matches what benchmark generates
	jsonData := `{"type":"user","message":{"role":"user","content":"Message 0 with some test content for searching"},"uuid":"uuid-0","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}`

	var msg schemas.SessionMessage
	err := json.Unmarshal([]byte(jsonData), &msg)
	if err != nil {
		t.Fatalf("Failed to unmarshal: %v", err)
	}

	fmt.Printf("Type: %s\n", msg.GetType())
	fmt.Printf("Content: %q\n", msg.GetContentText())
	fmt.Printf("UUID: %v\n", msg.GetUUID())
	fmt.Printf("SessionID: %v\n", msg.GetSessionID())

	// Debug the message field
	fmt.Printf("Message field: %+v\n", msg.Message)
	if msg.Message != nil {
		fmt.Printf("Message.Content type: %T\n", msg.Message.Content)
		fmt.Printf("Message.Content value: %+v\n", msg.Message.Content)
	}
}