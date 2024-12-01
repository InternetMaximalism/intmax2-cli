

import { ethers } from 'ethers';
import * as RollupArtifact from '../abi/Rollup.json';
import * as LiquidityArtifact from '../abi/Liquidity.json';

export async function deposit(privateKey: string, l1RpcUrl: string, liquidityContractAddress: string, l2RpcUrl: string, rollupContractAddress: string, amount: bigint, tokenType: number, tokenAddress: string, tokenId: string, pubkeySaltHash: string,) {
    const { liquidityContract, rollupContract } = await getContract(privateKey, l1RpcUrl, liquidityContractAddress, l2RpcUrl, rollupContractAddress);
    if (tokenType === 0) {
        await liquidityContract.depositNativeToken(pubkeySaltHash, { value: amount });
    } else if (tokenType === 1) {
        await liquidityContract.depositERC20(tokenAddress, pubkeySaltHash, amount);
    } else if (tokenType === 2) {
        await liquidityContract.depositERC721(tokenAddress, tokenId, pubkeySaltHash);
    } else if (tokenType === 3) {
        await liquidityContract.depositERC1155(tokenAddress, tokenId, pubkeySaltHash, amount);
    } else {
        throw new Error("Invalid token type");
    }
    const [isRegistered, tokenIndex] = await liquidityContract.getTokenIndex(tokenType, tokenAddress, tokenId);
    if (!isRegistered) {
        throw new Error("Token is not registered");
    }
    console.log("Deposited successfully");

    // following code is not used in testnet-alpha. Relay the deposits to the rollup contract
    const depositHash = getDepositHash(pubkeySaltHash, tokenIndex, amount);
    const tx = await rollupContract.processDeposits(0, [depositHash,]);
    await tx.wait();
    console.log("Deposits relayed to the rollup contract");
}

function getDepositHash(recipientSaltHash: string, tokenIndex: number, amount: bigint): string {
    return ethers.solidityPackedKeccak256(
        ['bytes32', 'uint32', 'uint256'],
        [recipientSaltHash, tokenIndex, amount]
    );
}

async function getContract(privateKey: string, l1RpcUrl: string, liquidityContractAddress: string, l2RpcUrl: string, rollupContractAddress: string,): Promise<{ liquidityContract: ethers.Contract, rollupContract: ethers.Contract }> {
    const l1Povider = new ethers.JsonRpcProvider(l1RpcUrl);
    const l1Wallet = new ethers.Wallet(privateKey, l1Povider)
    const liquidityContract = new ethers.Contract(
        liquidityContractAddress,
        LiquidityArtifact.abi,
        l1Wallet
    );
    const l2Provider = new ethers.JsonRpcProvider(l2RpcUrl);
    const l2Wallet = new ethers.Wallet(privateKey, l2Provider);
    const rollupContract = new ethers.Contract(
        rollupContractAddress,
        RollupArtifact.abi,
        l2Wallet
    );
    return { liquidityContract, rollupContract };
}

export async function getEthBalance(privateKey: string, rpcUrl: string): Promise<bigint> {
    const provider = new ethers.JsonRpcProvider(rpcUrl, undefined, {
        staticNetwork: true
    });
    const wallet = new ethers.Wallet(privateKey, provider);
    if (!wallet.provider) {
        throw new Error("Provider is not set");
    }
    const balance = await wallet.provider.getBalance(wallet.address);
    return balance;
}