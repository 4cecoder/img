#!/bin/bash

# Check if binary exists
if [ ! -f img ]; then
  echo "img binary not found. Please build it first with build.sh"
  exit 1
fi

# Install binary
sudo cp img /usr/local/bin

echo "img binary installed successfully"

