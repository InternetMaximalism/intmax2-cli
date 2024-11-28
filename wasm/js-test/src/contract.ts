

import { ethers } from 'ethers';
import * as RollupArtifact from '../abi/Rollup.json';
import * as LiquidityArtifact from '../abi/Liquidity.json';


export async function deposit(privateKey: string, l1RpcUrl: string, liquidityContractAddress: string, l2RpcUrl: string, rollupContractAddress: string, amount: bigint, tokenType: number, tokenAddress: string, tokenId: string, pubkeySaltHash: string,) {
    const { liquidityContract, rollupContract } = await getContract(privateKey, l1RpcUrl, liquidityContractAddress, l2RpcUrl, rollupContractAddress);

    if (tokenType === 0) {
        await liquidityContract.depositNativeToken(pubkeySaltHash, { value: amount });
    } else if (tokenType === 1) {
        await liquidityContract.depositERC20(tokenAddress, pubkeySaltHash, amount);
    } else {
        throw new Error("Not supported for NFT and other token types");
    }
    const [isRegistered, tokenIndex] = await liquidityContract.getTokenIndex(tokenType, tokenAddress, tokenId);
    if (!isRegistered) {
        throw new Error("Token is not registered");
    }

    // following code is not used in production. Rekay the deposits to the rollup contract
    // const tokenIndex = await liquidityContract.getTokenIndex(tokenType, tokenAddress, tokenId);
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
    const l1Povider = new ethers.JsonRpcProvider(l1RpcUrl, undefined, {
        staticNetwork: true
    });
    const l1Wallet = new ethers.Wallet(privateKey, l1Povider);
    const liquidityContract = new ethers.Contract(
        liquidityContractAddress,
        LiquidityArtifact.abi,
        l1Wallet
    );
    const l2Provider = new ethers.JsonRpcProvider(l2RpcUrl, undefined, {
        staticNetwork: true
    });
    const rollupContract = new ethers.Contract(
        rollupContractAddress,
        RollupArtifact.abi,
        l2Provider
    );
    return { liquidityContract, rollupContract };
}
