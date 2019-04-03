#!/bin/bash
set -e

echo "Please run openocd in another terminal window (you might need sudo)"

unameOut="$(uname -s)"
gdb-multiarch -iex 'add-auto-load-safe-path .' $1
