# Manual Mouse Support Testing Guide

## Test Environment

Test the mouse click support in different terminal emulators:

1. **macOS Terminal.app**
2. **iTerm2**
3. **VS Code integrated terminal**
4. **Alacritty**
5. **Kitty**

## Test Scenarios

### 1. Search Results List Mouse Click

1. Run the application with a search query:
   ```bash
   cargo run -- -i "test"
   ```

2. Once search results appear:
   - Click on different result items with the mouse
   - Verify the selection moves to the clicked item
   - Verify Enter key opens the detail view for the selected item

### 2. Session Viewer Mouse Click

1. From search results, press `Ctrl+S` to open session viewer
2. In the session viewer:
   - Click on different session messages
   - Verify the selection moves to the clicked message
   - Verify Enter opens message details

### 3. Edge Cases

Test these edge cases:

1. **Truncated vs Full Text Mode**:
   - Toggle with `Ctrl+T`
   - Test clicking in both modes
   - In full text mode, messages can span multiple lines

2. **Scrolling**:
   - Fill the screen with many results
   - Scroll down using keyboard
   - Click on visible items after scrolling
   - Verify correct item is selected

3. **Click Outside Content**:
   - Click on borders, title bar, status bar
   - Verify no action is taken

4. **Terminal Resize**:
   - Resize terminal while app is running
   - Test clicking after resize
   - Verify coordinates are correctly mapped

## Expected Behavior

- Left mouse button click selects the item at the clicked position
- Selection is immediate and visual feedback is shown
- Only clicks within the content area (not borders) should work
- Keyboard navigation continues to work alongside mouse

## Terminal Compatibility Notes

Some terminals may not support mouse events:
- SSH sessions may not forward mouse events
- Some terminal multiplexers (tmux, screen) may interfere
- Check terminal settings for mouse support options