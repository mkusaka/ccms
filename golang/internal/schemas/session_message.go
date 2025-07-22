package schemas

import (
	"encoding/json"
	"strings"
)

// BaseMessage contains fields common to most message types
type BaseMessage struct {
	ParentUUID  *string `json:"parentUuid"`
	IsSidechain bool    `json:"isSidechain"`
	UserType    string  `json:"userType"`
	CWD         string  `json:"cwd"`
	SessionID   string  `json:"sessionId"`
	Version     string  `json:"version"`
	UUID        string  `json:"uuid"`
	Timestamp   string  `json:"timestamp"`
}

// Content types
type Content struct {
	Type        string          `json:"type"`
	Text        string          `json:"text,omitempty"`
	ID          string          `json:"id,omitempty"`
	Name        string          `json:"name,omitempty"`
	Input       json.RawMessage `json:"input,omitempty"`
	ToolUseID   string          `json:"tool_use_id,omitempty"`
	Content     json.RawMessage `json:"content,omitempty"`
	IsError     *bool           `json:"is_error,omitempty"`
	Thinking    string          `json:"thinking,omitempty"`
	Signature   string          `json:"signature,omitempty"`
	Source      *ImageSource    `json:"source,omitempty"`
}

type ImageSource struct {
	Type      string  `json:"type"`
	Data      *string `json:"data,omitempty"`
	MediaType *string `json:"media_type,omitempty"`
}

// Usage information
type Usage struct {
	InputTokens               uint32          `json:"input_tokens"`
	CacheCreationInputTokens  uint32          `json:"cache_creation_input_tokens"`
	CacheReadInputTokens      uint32          `json:"cache_read_input_tokens"`
	OutputTokens              uint32          `json:"output_tokens"`
	ServiceTier               *string         `json:"service_tier,omitempty"`
	ServerToolUse             *ServerToolUse  `json:"server_tool_use,omitempty"`
}

type ServerToolUse struct {
	WebSearchRequests uint32 `json:"web_search_requests"`
}

// Message content structures
type UserMessageContent struct {
	Role    string      `json:"role"`
	Content interface{} `json:"content"` // Can be string or []Content
}

type AssistantMessageContent struct {
	ID           string    `json:"id"`
	Type         string    `json:"type"`
	Role         string    `json:"role"`
	Model        string    `json:"model"`
	Content      []Content `json:"content"`
	StopReason   *string   `json:"stop_reason"`
	StopSequence *string   `json:"stop_sequence"`
	Usage        Usage     `json:"usage"`
}

// SessionMessage represents all possible message types
type SessionMessage struct {
	Type string `json:"type"`

	// Summary fields
	Summary  string  `json:"summary,omitempty"`
	LeafUUID *string `json:"leafUuid,omitempty"`

	// Common fields (for system, user, assistant)
	BaseMessage

	// System specific
	Content  string  `json:"content,omitempty"`
	IsMeta   *bool   `json:"isMeta,omitempty"`
	ToolUseID *string `json:"toolUseID,omitempty"`
	Level     *string `json:"level,omitempty"`

	// User specific
	Message           *UserMessageContent `json:"message,omitempty"`
	IsCompactSummary  *bool              `json:"isCompactSummary,omitempty"`
	ToolUseResult     json.RawMessage    `json:"toolUseResult,omitempty"`

	// Assistant specific
	AssistantMessage  *AssistantMessageContent `json:"message,omitempty"`
	IsAPIErrorMessage *bool                   `json:"isApiErrorMessage,omitempty"`

	// Shared optional fields
	GitBranch  *string `json:"gitBranch,omitempty"`
	RequestID  *string `json:"requestId,omitempty"`
}

// Helper methods
func (m *SessionMessage) GetType() string {
	return m.Type
}

func (m *SessionMessage) GetContentText() string {
	switch m.Type {
	case "summary":
		return m.Summary
	case "system":
		return m.Content
	case "user":
		if m.Message == nil {
			return ""
		}
		var texts []string
		
		switch content := m.Message.Content.(type) {
		case string:
			texts = append(texts, content)
		case []interface{}:
			for _, item := range content {
				if contentMap, ok := item.(map[string]interface{}); ok {
					if contentType, ok := contentMap["type"].(string); ok {
						switch contentType {
						case "text":
							if text, ok := contentMap["text"].(string); ok {
								texts = append(texts, text)
							}
						case "tool_result":
							if toolContent, ok := contentMap["content"]; ok {
								switch tc := toolContent.(type) {
								case string:
									texts = append(texts, tc)
								case []interface{}:
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
			}
		}
		return strings.Join(texts, "\n")
		
	case "assistant":
		if m.AssistantMessage == nil {
			return ""
		}
		var texts []string
		for _, content := range m.AssistantMessage.Content {
			switch content.Type {
			case "text":
				texts = append(texts, content.Text)
			case "thinking":
				texts = append(texts, content.Thinking)
			}
		}
		return strings.Join(texts, "\n")
	}
	return ""
}

func (m *SessionMessage) GetUUID() *string {
	switch m.Type {
	case "summary":
		return m.LeafUUID
	default:
		return &m.UUID
	}
}

func (m *SessionMessage) GetTimestamp() *string {
	if m.Type == "summary" {
		return nil
	}
	return &m.Timestamp
}

func (m *SessionMessage) GetSessionID() *string {
	if m.Type == "summary" {
		return nil
	}
	return &m.SessionID
}

func (m *SessionMessage) HasToolUse() bool {
	if m.Type != "assistant" || m.AssistantMessage == nil {
		return false
	}
	for _, content := range m.AssistantMessage.Content {
		if content.Type == "tool_use" {
			return true
		}
	}
	return false
}

func (m *SessionMessage) HasThinking() bool {
	if m.Type != "assistant" || m.AssistantMessage == nil {
		return false
	}
	for _, content := range m.AssistantMessage.Content {
		if content.Type == "thinking" {
			return true
		}
	}
	return false
}

// UnmarshalJSON handles the polymorphic nature of SessionMessage
func (m *SessionMessage) UnmarshalJSON(data []byte) error {
	// Use alias to avoid infinite recursion
	type Alias SessionMessage
	var aux Alias
	
	if err := json.Unmarshal(data, &aux); err != nil {
		return err
	}
	
	*m = SessionMessage(aux)
	
	// Fix message field for assistant messages
	if m.Type == "assistant" && aux.AssistantMessage == nil {
		// Try to unmarshal message field into AssistantMessage
		var temp struct {
			Message AssistantMessageContent `json:"message"`
		}
		if err := json.Unmarshal(data, &temp); err == nil {
			m.AssistantMessage = &temp.Message
		}
	}
	
	return nil
}