package main

import (
	"bytes"
	"fmt"
	"os"
	"strings"

	"github.com/minio/simdjson-go"
)

func main() {
	// Read a sample file
	// Find one file
	data, err := os.ReadFile("/Users/masatomokusaka/.claude/projects/-Users-masatomokusaka-src-github-com-mkusaka-bookmark-agent--git-tmp-worktrees-20250720-234420-ci/15db177e-c1c9-4413-9c79-22cc9ba6275a.jsonl")
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading file: %v\n", err)
		return
	}
	
	lines := bytes.Split(data, []byte{'\n'})
	fmt.Fprintf(os.Stderr, "File has %d lines\n", len(lines))
	
	count := 0
	for i, line := range lines {
		if len(line) == 0 {
			continue
		}
		
		if i > 5 {
			break // Just check first few lines
		}
		
		// Parse JSON line
		pj, err := simdjson.Parse(line, nil)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Line %d: Parse error: %v\n", i, err)
			continue
		}
		
		// Get iterator and advance to root
		iter := pj.Iter()
		typ := iter.Advance()
		fmt.Fprintf(os.Stderr, "\nLine %d: Root type: %v\n", i, typ)
		
		if typ != simdjson.TypeObject {
			continue
		}
		
		// Convert to object
		obj, err := iter.Object(nil)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Line %d: Object error: %v\n", i, err)
			continue
		}
		
		// Get message type
		var msgType string
		if typeElem := obj.FindKey("type", nil); typeElem != nil {
			msgType, err = typeElem.Iter.String()
			fmt.Fprintf(os.Stderr, "  Type: %s (err: %v)\n", msgType, err)
		}
		
		// Try to get content based on type
		if msgType == "user" || msgType == "assistant" {
			// Get message object
			if msgElem := obj.FindKey("message", nil); msgElem != nil {
				fmt.Fprintf(os.Stderr, "  Found 'message' key\n")
				msgIter := msgElem.Iter
				msgTyp := msgIter.Type()
				fmt.Fprintf(os.Stderr, "  Message type: %v\n", msgTyp)
				
				if msgTyp == simdjson.TypeObject {
					msgObj, err := msgIter.Object(nil)
					if err != nil {
						fmt.Fprintf(os.Stderr, "  Message object error: %v\n", err)
					} else {
						// Get content from message
						if contentElem := msgObj.FindKey("content", nil); contentElem != nil {
							contentType := contentElem.Iter.Type()
							fmt.Fprintf(os.Stderr, "  Content type: %v\n", contentType)
							
							if contentType == simdjson.TypeString {
								content, err := contentElem.Iter.String()
								fmt.Fprintf(os.Stderr, "  String content: %q (err: %v)\n", strings.TrimSpace(content)[:100], err)
								if strings.Contains(strings.ToLower(content), "error") {
									count++
								}
							} else if contentType == simdjson.TypeArray {
								fmt.Fprintf(os.Stderr, "  Content is array\n")
								arr, err := contentElem.Iter.Array(nil)
								if err != nil {
									fmt.Fprintf(os.Stderr, "  Array error: %v\n", err)
								} else {
									var texts []string
									arr.ForEach(func(i simdjson.Iter) {
										itemType := i.Type()
										fmt.Fprintf(os.Stderr, "    Array item type: %v\n", itemType)
										if itemType == simdjson.TypeObject {
											itemObj, err := i.Object(nil)
											if err == nil {
												if typeElem := itemObj.FindKey("type", nil); typeElem != nil {
													itemTypeStr, _ := typeElem.Iter.String()
													fmt.Fprintf(os.Stderr, "    Item type: %s\n", itemTypeStr)
												}
											}
										}
									})
									fmt.Fprintf(os.Stderr, "  Found %d texts in array\n", len(texts))
								}
							}
						}
					}
				}
			}
		}
	}
	
	fmt.Printf("Found %d messages with 'error'\n", count)
}