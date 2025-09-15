#!/bin/bash

# Install script for img

set -e

echo "Installing img..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed. Please install Rust first:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build the project in release mode
echo "Building img..."
cargo build --release

# Create local bin directory if it doesn't exist
mkdir -p ~/.local/bin

# Install the binary
echo "Installing binary to ~/.local/bin..."
cp target/release/img ~/.local/bin/

# Add to PATH if not already there
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "Adding ~/.local/bin to PATH..."
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
    echo "Please restart your shell or run 'source ~/.bashrc' to update PATH"
fi

echo "Installation complete! You can now run 'img' from anywhere."