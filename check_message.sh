#!/bin/bash

# Check specific message and related messages
UUID="36692432-493c-494c-81bf-3bdd04ac09eb"
FILE="/Users/masatomokusaka/.claude/projects/-Users-masatomokusaka-src-github-com-mkusaka-ccms--git-tmp-worktrees-20250730-020517-truncate/efd5d372-f77c-4563-9a33-b6fec33111b8.jsonl"

echo "=== Target Message ==="
jq --arg uuid "$UUID" 'select(.uuid == $uuid)' "$FILE" | jq .

echo -e "\n=== Parent Message ==="
PARENT_UUID=$(jq --arg uuid "$UUID" -r 'select(.uuid == $uuid) | .parentUuid' "$FILE")
jq --arg uuid "$PARENT_UUID" 'select(.uuid == $uuid)' "$FILE" | jq .

echo -e "\n=== Messages with same toolUseID ==="
TOOL_USE_ID=$(jq --arg uuid "$UUID" -r 'select(.uuid == $uuid) | .toolUseID' "$FILE")
jq --arg id "$TOOL_USE_ID" 'select(.toolUseID == $id) | {uuid, timestamp, content: (.content | if type == "string" then (.[0:100] + "...") else . end)}' "$FILE"

echo -e "\n=== All system messages around the same time ==="
TIMESTAMP=$(jq --arg uuid "$UUID" -r 'select(.uuid == $uuid) | .timestamp' "$FILE")
jq --arg ts "$TIMESTAMP" 'select(.type == "system" and (.timestamp | . >= ($ts | sub("T.*"; "T17:36:20")) and . <= ($ts | sub("T.*"; "T17:36:30")))) | {uuid, timestamp, content: (.content | if type == "string" then (.[0:100] + "...") else . end)}' "$FILE"