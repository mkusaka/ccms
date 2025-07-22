package schemas

import (
	"encoding/json"
	"strings"
)

// SimpleMessage is a simplified version for benchmarking
type SimpleMessage struct {
	Type      string          `json:"type"`
	UUID      string          `json:"uuid"`
	Timestamp string          `json:"timestamp"`
	SessionID string          `json:"sessionId"`
	Message   json.RawMessage `json:"message,omitempty"`
	Content   string          `json:"content,omitempty"`
	Summary   string          `json:"summary,omitempty"`
	LeafUUID  string          `json:"leafUuid,omitempty"`
	
	// Cached content text
	contentText string
}

// GetType returns the message type
func (m *SimpleMessage) GetType() string {
	return m.Type
}

// GetContentText returns the text content of the message
func (m *SimpleMessage) GetContentText() string {
	if m.contentText != "" {
		return m.contentText
	}
	
	switch m.Type {
	case "summary":
		m.contentText = m.Summary
	case "system":
		m.contentText = m.Content
	case "user", "assistant":
		// Try to extract content from message field
		if len(m.Message) == 0 {
			return ""
		}
		
		var msgObj map[string]interface{}
		if err := json.Unmarshal(m.Message, &msgObj); err != nil {
			return ""
		}
		
		// Extract content field
		if content, ok := msgObj["content"]; ok {
			switch c := content.(type) {
			case string:
				m.contentText = c
			case []interface{}:
				var texts []string
				for _, item := range c {
					if itemMap, ok := item.(map[string]interface{}); ok {
						itemType, _ := itemMap["type"].(string)
						switch itemType {
						case "text":
							if text, ok := itemMap["text"].(string); ok {
								texts = append(texts, text)
							}
						case "thinking":
							if thinking, ok := itemMap["thinking"].(string); ok {
								texts = append(texts, thinking)
							}
						case "tool_result":
							// Handle tool result content
							if toolContent, ok := itemMap["content"]; ok {
								switch tc := toolContent.(type) {
								case string:
									texts = append(texts, tc)
								case []interface{}:
									// Handle array of text items
									for _, textItem := range tc {
										if textMap, ok := textItem.(map[string]interface{}); ok {
											if text, ok := textMap["text"].(string); ok {
												texts = append(texts, text)
											}
										}
									}
								}
							}
						}
					}
				}
				m.contentText = strings.Join(texts, "\n")
			}
		}
	}
	
	return m.contentText
}

// GetUUID returns the UUID
func (m *SimpleMessage) GetUUID() *string {
	if m.Type == "summary" && m.LeafUUID != "" {
		return &m.LeafUUID
	}
	if m.UUID != "" {
		return &m.UUID
	}
	return nil
}

// GetTimestamp returns the timestamp
func (m *SimpleMessage) GetTimestamp() *string {
	if m.Type == "summary" || m.Timestamp == "" {
		return nil
	}
	return &m.Timestamp
}

// GetSessionID returns the session ID
func (m *SimpleMessage) GetSessionID() *string {
	if m.Type == "summary" || m.SessionID == "" {
		return nil
	}
	return &m.SessionID
}