

import { ethers } from 'ethers';
import * as RollupArtifact from '../abi/Rollup.json';
import * as LiquidityArtifact from '../abi/Liquidity.json';
import dotenv from 'dotenv';
import { url } from 'inspector';
import { cleanEnv, num, str } from 'envalid';
dotenv.config();

const env = cleanEnv(process.env, {
    // L1 configurations
    L1_RPC_URL: url(),
    L1_CHAIN_ID: num(),
    LIQUIDITY_CONTRACT_ADDRESS: str(),

    // L2 configurations
    L2_RPC_URL: url(),
    L2_CHAIN_ID: num(),
    ROLLUP_CONTRACT_ADDRESS: str(),
    ROLLUP_CONTRACT_DEPLOYED_BLOCK_NUMBER: num(),
});


export async function deposit(privateKey: string, l1RpcUrl: string, liquidityContractAddress: string, l2RpcUrl: string, rollupContractAddress: string, amount: bigint, tokenType: number, tokenAddress: string, tokenId: string, pubkeySaltHash: string,) {
    const { liquidityContract, rollupContract } = await getContract(privateKey, l1RpcUrl, liquidityContractAddress, l2RpcUrl, rollupContractAddress);
    if (tokenType === 0) {
        await liquidityContract.depositNativeToken(pubkeySaltHash, { value: amount });
    } else if (tokenType === 1) {
        await liquidityContract.depositERC20(tokenAddress, pubkeySaltHash, amount);
    } else {
        throw new Error("Not supported for NFT and other token types");
    }

    // following code is not used in production. Rekay the deposits to the rollup contract
    const tokenIndex = await liquidityContract.getTokenIndex(tokenType, tokenAddress, tokenId);
    const depositHash = getDepositHash(pubkeySaltHash, tokenIndex, amount);
    const tx = await rollupContract.processDeposits(0, [depositHash]);
    await tx.wait();
}

function getDepositHash(recipientSaltHash: string, tokenIndex: number, amount: bigint): string {
    return ethers.solidityPackedKeccak256(
        ['bytes32', 'uint32', 'uint256'],
        [recipientSaltHash, tokenIndex, amount]
    );
}

async function getContract(privateKey: string, l1RpcUrl: string, liquidityContractAddress: string, l2RpcUrl: string, rollupContractAddress: string,): Promise<{ liquidityContract: ethers.Contract, rollupContract: ethers.Contract }> {
    const l1Povider = new ethers.JsonRpcProvider(l1RpcUrl);

    const l1Wallet = new ethers.Wallet(privateKey, l1Povider);
    const liquidityContract = new ethers.Contract(
        liquidityContractAddress,
        LiquidityArtifact.abi,
        l1Wallet
    );

    const l2Provider = new ethers.JsonRpcProvider(l2RpcUrl);
    const rollupContract = new ethers.Contract(
        rollupContractAddress,
        RollupArtifact.abi,
        l2Provider
    );

    return { liquidityContract, rollupContract };
}


async function main() {

}