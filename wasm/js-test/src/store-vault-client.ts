import axios, { AxiosInstance } from 'axios';
import { cleanEnv, url } from 'envalid';
import { hexToBigInt } from './utils';

export interface MetaData {
    uuid: string;
    timestamp: number;
}

interface GetDataResponse {
    data: [MetaData, Uint8Array] | null;
}

interface GetDataAllAfterResponse {
    data: Array<[MetaData, Uint8Array]>;
}

interface GetDataQuery {
    uuid: string;
}

interface GetDataAllAfterQuery {
    pubkey: string;
    timestamp: number;
}

export class StoreVaultClient {
    private client: AxiosInstance;
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
        this.client = axios.create({
            baseURL: baseUrl,
        });
    }

    async getData(
        type: 'deposit' | 'withdrawal' | 'transfer' | 'tx',
        uuid: string
    ): Promise<[MetaData, Uint8Array] | null> {
        try {
            const query: GetDataQuery = { uuid };
            const response = await this.client.get<GetDataResponse>(
                `/store-vault-server/${type}/get`,
                { params: query }
            );
            return response.data.data;
        } catch (error) {
            if (axios.isAxiosError(error)) {
                throw new Error(`Server Error: ${error.message}`);
            }
            throw error;
        }
    }

    async getAllAfter(
        type: 'deposit' | 'withdrawal' | 'transfer' | 'tx',
        pubkey: string,
        timestamp: number
    ): Promise<Array<[MetaData, Uint8Array]>> {
        try {
            const query: GetDataAllAfterQuery = { pubkey: hexToBigInt(pubkey).toString(), timestamp };
            const response = await this.client.get<GetDataAllAfterResponse>(
                `/store-vault-server/${type}/get-all-after`,
                { params: query }
            );
            return response.data.data;
        } catch (error) {
            if (axios.isAxiosError(error)) {
                throw new Error(`Server Error: ${error.message}`);
            }
            throw error;
        }
    }
}

