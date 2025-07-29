# Add mouse click support for session message selection

## Summary

This PR implements mouse click support for selecting messages in both the search results list and session viewer, addressing issue #112. Users can now click on messages to select them, complementing the existing keyboard navigation.

## Changes

### Core Implementation

1. **Extended Component trait**
   - Added `handle_mouse()` method with default implementation
   - Allows components to optionally handle mouse events

2. **Updated main event loop**
   - Enabled mouse capture in terminal setup
   - Added mouse event processing alongside keyboard events
   - Track component rendering areas for accurate click mapping

3. **ListViewer mouse support**
   - Implements click-to-select for list items
   - Handles both truncated and full-text display modes
   - Correctly accounts for scroll offset
   - Validates clicks are within component boundaries

4. **Integration with ResultList and SessionViewer**
   - Mouse clicks update selection without triggering navigation
   - Maintains consistency with keyboard behavior (select, then Enter to view)

### Testing

- Added comprehensive tests for mouse event handling
- Tests cover edge cases: boundary clicks, scrolling, filtering
- All existing tests continue to pass

## Usage

- **Left-click** on any visible message to select it
- **Enter key** to view details of selected message (unchanged)
- Keyboard navigation continues to work as before

## Technical Considerations

### Known Trade-off: Terminal Text Selection

**Important**: Enabling mouse support in the terminal means that the terminal's native text selection (click and drag to select text) is disabled while the application is running. This is a fundamental limitation of terminal mouse capture.

Users who need to copy text from the terminal have these options:
1. Use the built-in copy commands (c/C for text/JSON)
2. Temporarily disable mouse support (would require a toggle feature)
3. Use terminal-specific modifier keys (e.g., hold Shift while selecting in some terminals)

This trade-off should be considered when deciding whether to merge this feature. Some users may prefer keyboard-only navigation to maintain text selection capability.

## Future Enhancements

- Add a toggle to enable/disable mouse support at runtime
- Support scroll wheel for navigation
- Right-click context menus
- Extend mouse support to other components (search bar, etc.)

## Compatibility

- Mouse support degrades gracefully on terminals without mouse capability
- No changes to existing keyboard navigation
- Backwards compatible with current user workflows

Closes #112