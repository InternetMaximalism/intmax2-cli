import * as dotenv from 'dotenv';
import { cleanEnv, num, str, url } from 'envalid';
import { WithdrawalServerClient } from './withdrawal-status';
import { generate_intmax_account_from_eth_key } from '../pkg/intmax2_wasm_lib';
import { ethers } from 'ethers';
import { claimWithdrawals } from './contract';
dotenv.config();

const env = cleanEnv(process.env, {
    USER_ETH_PRIVATE_KEY: str(),
    ENV: str(),

    WITHDRAWAL_SERVER_BASE_URL: url(),

    // L1 configurations
    L1_RPC_URL: url(),
    L1_CHAIN_ID: num(),
    LIQUIDITY_CONTRACT_ADDRESS: str(),
});

async function main() {
    const ethKey = env.USER_ETH_PRIVATE_KEY;
    const ethAddress = new ethers.Wallet(ethKey).address;
    console.log("ethAddress: ", ethAddress);

    const key = await generate_intmax_account_from_eth_key(ethKey);
    const publicKey = key.pubkey;
    const privateKey = key.privkey;
    console.log("privateKey: ", privateKey);
    console.log("publicKey: ", publicKey);

    // print withdrawal status 
    const withdrawalClient = new WithdrawalServerClient(env.WITHDRAWAL_SERVER_BASE_URL);
    const withdrawalStatus = await withdrawalClient.getWithdrawalInfo(publicKey);
    console.log("Withdrawal status: ", withdrawalStatus);

    let needClaimWithdrawals = [];
    for (const status of withdrawalStatus) {
        // claim withdrawal if needed
        if (status.status === 3) {
            needClaimWithdrawals.push(status.withdrawal);
        }
    }
    await claimWithdrawals(privateKey, env.L1_RPC_URL, env.LIQUIDITY_CONTRACT_ADDRESS, needClaimWithdrawals);
}

main().then(() => {
    process.exit(0);
}).catch((err) => {
    console.error(err);
    process.exit(1);
});