#!/bin/bash
# clean_debug_runs.sh
# Cleans up debug run files.
# Usage:
#   ./clean_debug_runs.sh        # deletes all debug files
#   ./clean_debug_runs.sh 7      # deletes debug files older than 7 days

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../.battler-debug-test-runs"

if [ ! -d "$DIR" ]; then
    echo "No .battler-debug-test-runs directory found at $DIR"
    exit 0
fi

if [ -z "$1" ]; then
    echo "Cleaning all debug files in $DIR..."
    rm -rf "$DIR"/*
    echo "Done."
else
    echo "Cleaning debug files in $DIR older than $1 days..."
    find "$DIR" -name "*.json" -mtime +"$1" -delete
    echo "Done."
fi
