#\!/bin/bash

echo "Testing iocraft interactive mode..."
echo "Initial query: 'claude'"
echo "Pattern: default Claude pattern"
echo
echo "To test:"
echo "1. The search bar should show 'claude'"
echo "2. Results should appear below"
echo "3. Use arrow keys to navigate"
echo "4. Press 'q' to quit"
echo
echo "Starting in 3 seconds..."
sleep 3

cargo run --release -- --interactive --iocraft "claude"
