# Async Session List Search Implementation

## Summary

This implementation adds asynchronous search functionality to the session list, similar to how message search works.

## Changes Made

### 1. Added Async State Fields to SessionListState
- `filtered_sessions`: Vec<SessionInfo> - Stores the filtered results
- `query`: String - The search query
- `is_searching`: bool - Indicates if search is in progress
- `current_search_id`: u64 - Used to track the current search operation

### 2. Added New Message Types
- `SessionListQueryChanged(String)` - Triggered when the user types in the session list search
- `SessionListSearchRequested` - Triggered when debounce timer expires
- `SessionListSearchCompleted(Vec<SessionInfo>)` - Triggered when search completes

### 3. Added New Command Types
- `ExecuteSessionListSearch` - Execute the async search
- `ScheduleSessionListSearch(u64)` - Schedule search with debounce delay

### 4. Updated SessionList UI Component
- Added a search bar at the top of the session list
- Shows "searching..." status during search
- Displays filtered results based on search query
- Search is performed on session_id, first_message, and summary fields

### 5. Added Async Search Logic
- Search is executed asynchronously using `blocking::unblock`
- 300ms debounce to avoid too many searches while typing
- Case-insensitive search across session metadata
- Empty query shows all sessions

## Key Features

1. **Non-blocking Search**: Uses async execution to prevent UI freezing
2. **Debounced Input**: 300ms delay before executing search
3. **Visual Feedback**: Shows "[typing...]" and "[searching...]" states
4. **Comprehensive Search**: Searches in session ID, first message, and summary
5. **Consistent UX**: Works the same way as message search

## Testing

All existing tests have been updated to work with the new async session list search functionality.