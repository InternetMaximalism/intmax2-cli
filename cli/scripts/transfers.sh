#!/bin/bash

# Default values
DEFAULT_PUBLIC_KEY="0x$(printf '0%.0s' {1..64})"  # 0x0...0 (64 zeros)
DEFAULT_AMOUNT=1
DEFAULT_TOKEN_INDEX=0
DEFAULT_PARALLEL_JOBS=4
DEFAULT_NUM_KEYS="4"

# Parse command line arguments
public_key=$DEFAULT_PUBLIC_KEY
amount=$DEFAULT_AMOUNT
token_index=$DEFAULT_TOKEN_INDEX
parallel_jobs=$DEFAULT_PARALLEL_JOBS
num_keys=$DEFAULT_NUM_KEYS

# Help function
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo "Options:"
    echo "  --to ADDRESS          Destination address (default: 0x0...0)"
    echo "  --amount VALUE        Amount to transfer (default: 1)"
    echo "  --token-index INDEX   Token index (default: 0)"
    echo "  --parallel JOBS       Number of parallel jobs (default: 4)"
    echo "  --num-keys NUMBER     Number of keys to process (default: all)"
    echo "  --help               Show this help message"
}

# Process command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --to)
            public_key="$2"
            shift 2
            ;;
        --amount)
            amount="$2"
            shift 2
            ;;
        --token-index)
            token_index="$2"
            shift 2
            ;;
        --parallel)
            parallel_jobs="$2"
            shift 2
            ;;
        --num-keys)
            num_keys="$2"
            shift 2
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown parameter: $1"
            show_help
            exit 1
            ;;
    esac
done

# Check if private_keys.txt exists
if [ ! -f private_keys.txt ]; then
    echo "Error: private_keys.txt not found"
    exit 1
fi

# Function to process a single transfer
process_key() {
    local private_key=$1
    local public_key=$2
    local amount=$3
    local token_index=$4
    
    echo "Processing transfer with private key: ${private_key:0:10}..."
    cargo run -r -- transfer \
        --private-key "$private_key" \
        --to "$public_key" \
        --amount "$amount" \
        --token-index "$token_index"
}
export -f process_key

# Prepare input based on num_keys parameter
if [ "$num_keys" = "all" ]; then
    input_keys="cat private_keys.txt"
else
    input_keys="head -n $num_keys private_keys.txt"
fi

# Execute transfers in parallel
eval "$input_keys" | \
    parallel --will-cite -j "$parallel_jobs" process_key {} "$public_key" "$amount" "$token_index"

echo "Transfer process completed"
