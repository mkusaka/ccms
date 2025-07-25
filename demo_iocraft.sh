#!/bin/bash

echo "=== Iocraft Interactive Mode Demo ==="
echo
echo "This demonstrates the completed iocraft migration."
echo
echo "Features implemented:"
echo "✅ Full TextInput with all editing shortcuts"
echo "✅ Keyboard navigation (arrow keys, j/k, Tab for role filter)"
echo "✅ Search functionality with Claude session files"
echo "✅ Secure clipboard integration"
echo "✅ Error handling (no unwrap() calls)"
echo "✅ Unicode support (Japanese text)"
echo
echo "Controls:"
echo "- Type to search"
echo "- Arrow keys or j/k to navigate results"
echo "- Enter to view result details"
echo "- Tab to cycle role filter (user/assistant/system)"
echo "- Esc to go back"
echo "- Ctrl+C twice to exit"
echo
echo "Starting interactive mode with query 'test' in 3 seconds..."
sleep 3

cargo run --release -- --interactive --iocraft "test"