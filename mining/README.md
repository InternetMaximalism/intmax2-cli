# Mining

## Privacy Staking

1. Deposit 1 ETH into the INTMAX network.
2. Keep the deposit within the network until the predetermined holding period has elapsed.
3. Withdraw an amount between 0.99 and 1 ETH.
4. Provide proof of elapsed time (PoET) using ZKP.
5. Receive the reward.

## PoET

### 1. Verification Details

Verify that a transfer was made from Ethereum address A to Ethereum address B via INTMAX address C.

### 2. Withdrawal Proof for Address C → B (TxInclusionCircuit, SpentCircuit)

Using respective circuits, verify whether the transaction was correctly reflected and that the final recipient is address B.

### 3. Deposit Proof for Address A → C (ReceiveDepositCircuit)

Use deposit_merkle_proof and other mechanisms to prove that a deposit from A to C exists.
Only the user who knows the depositSalt can provide this proof.

### 4. Address A and Address B Must Be Different

INTMAX address C is generated using the same private key as Ethereum address A, while Ethereum address B must be a different address chosen by the user.
