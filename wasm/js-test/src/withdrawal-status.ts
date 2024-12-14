import axios from 'axios';
import { hexToBigInt } from './utils';

export type Address = string; // Ethereum address
export type U256 = string; // Big number as string

export enum WithdrawalStatus {
    Requested = 0,
    Relayed = 1,
    Success = 2,
    NeedClaim = 3,
    Failed = 4,
}

export interface Withdrawal {
    recipient: Address;
    tokenIndex: number;
    amount: U256;
    nullifier: string;
}

export interface WithdrawalInfo {
    status: WithdrawalStatus;
    withdrawal: Withdrawal;
}

export interface ContractWithdrawal {
    recipient: Address;
    tokenIndex: number;
    amount: U256;
    id: number;
}

export class ServerError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ServerError';
    }
}

export class WithdrawalServerClient {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }

    public async getWithdrawalInfo(pubkey: string): Promise<WithdrawalInfo[]> {
        try {
            const response = await axios.get<{ withdrawalInfo: WithdrawalInfo[] }>(
                `${this.baseUrl}/withdrawal-server/get-withdrawal-info`,
                {
                    params: {
                        pubkey: hexToBigInt(pubkey).toString(),
                        signature: ["0x0000000000000000000000000000000000000000000000000000000000000000", "0x0000000000000000000000000000000000000000000000000000000000000000", "0x0000000000000000000000000000000000000000000000000000000000000000", "0x0000000000000000000000000000000000000000000000000000000000000000"]
                    }
                }
            );
            return response.data.withdrawalInfo;
        } catch (error) {
            if (axios.isAxiosError(error)) {
                throw new ServerError(
                    error.response?.data?.message || 'Failed to get withdrawal info'
                );
            }
            throw error;
        }
    }


    public async getWithdrawalInfoByRecipient(
        recipient: Address
    ): Promise<WithdrawalInfo[]> {
        try {
            const response = await axios.get<{ withdrawalInfo: WithdrawalInfo[] }>(
                `${this.baseUrl}/withdrawal-server/get-withdrawal-info-by-recipient`,
                {
                    params: { recipient }
                }
            );
            return response.data.withdrawalInfo;
        } catch (error) {
            if (axios.isAxiosError(error)) {
                throw new ServerError(
                    error.response?.data?.message || 'Failed to get withdrawal info by recipient'
                );
            }
            throw error;
        }
    }
}