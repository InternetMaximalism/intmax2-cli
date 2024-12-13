#!/bin/bash
(

# Default values and check for command line arguments
N=${1:-64}           # Number of keys (default: 64)
AMOUNT=${2:-10}      # Amount for each transfer (default: 10)
TOKEN_INDEX=${3:-0}  # Token index (default: 0)

# Output files
CSV_FILE="transfers.csv"
PRIVATE_KEYS_FILE="private_keys.txt"

# Display configuration
echo "Configuration:"
echo "  Number of keys: $N"
echo "  Amount: $AMOUNT"
echo "  Token Index: $TOKEN_INDEX"
echo "-------------------"

# Create CSV header
echo "recipient,amount,tokenIndex" > $CSV_FILE

# Clear private keys file
> $PRIVATE_KEYS_FILE

# Generate N key pairs and create CSV entries
for i in $(seq 1 $N); do
    # Run the command and capture output
    output=$(cargo run -r -- generate-key)
    
    # Extract private and public keys using regex
    private_key=$(echo "$output" | grep "Private key:" | sed 's/.*: \(0x.*\)/\1/')
    public_key=$(echo "$output" | grep "Public key:" | sed 's/.*: \(0x.*\)/\1/')
    
    # Append to CSV file
    echo "$public_key,$AMOUNT,$TOKEN_INDEX" >> $CSV_FILE
    
    # Append to private keys file
    echo "$private_key" >> $PRIVATE_KEYS_FILE
    
    # Show progress
    echo "Generated key pair $i of $N"
done

echo "Done! Check $CSV_FILE and $PRIVATE_KEYS_FILE"
)
