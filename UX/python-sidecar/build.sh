#!/bin/bash
# Build Python sidecar for current platform

set -e

echo "Building FontAtlas Python Sidecar..."

# Install PyInstaller if not present
pip install pyinstaller

# Build for current platform
cd "$(dirname "$0")"
pyinstaller sidecar.spec

# Copy to binaries directory
mkdir -p ../binaries
cp dist/fontatlas-parser ../binaries/

echo "✅ Sidecar built: ../binaries/fontatlas-parser"
echo ""
echo "Test it:"
echo "  echo '{\"command\": \"ping\"}' | ../binaries/fontatlas-parser"
