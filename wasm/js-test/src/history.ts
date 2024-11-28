import { decrypt_deposit_data, decrypt_transfer_data, decrypt_tx_data, JsDepositData, JsTransferData, JsTxData, JsUserData } from "../pkg/intmax2_wasm_lib";
import { StoreVaultClient } from "./store-vault-client";

export async function printHistory(store_vault_server_base_url: string, privateKey: string, userData: JsUserData) {
    const storeVaultClient = new StoreVaultClient(store_vault_server_base_url);

    console.log("Deposit History:");
    const depositHistory = await fetchDepositHistory(storeVaultClient, privateKey, userData);
    for (const deposit of depositHistory) {
        console.log(deposit);
    }

    const receiveHistory = await fetchReceiveHistory(storeVaultClient, privateKey, userData);
    console.log("Receive History:");
    for (const receive of receiveHistory) {
        console.log(receive);
    }

    const sendHistory = await fetchSendHistory(storeVaultClient, privateKey, userData);
    console.log("Send History:");
    for (const send of sendHistory) {
        console.log(send);
    }
}


interface Deposit {
    amount: string;
    token_type: number;
    token_address: string;
    token_id: string,
    is_rejected: boolean;
    timestamp: number | null; // null if not yet confirmed
}

export async function fetchDepositHistory(storeVaultClient: StoreVaultClient, privateKey: string, userData: JsUserData): Promise<Deposit[]> {
    const allData = await storeVaultClient.getAllAfter("deposit", userData.pubkey, 0);
    const processedUuids = userData.processed_deposit_uuids;

    const history = [];
    for (const [metaData, data] of allData) {
        // decrypt data if possible 
        let decrypted: JsDepositData;
        try {
            decrypted = await decrypt_deposit_data(privateKey, data);
        } catch (error) {
            console.log("Error decrypting data: ", error);
            continue; // just ignore invalid data
        }

        let deposit: Deposit;
        if (BigInt(metaData.timestamp) <= userData.deposit_lpt) {
            if (!processedUuids.includes(metaData.uuid)) {
                deposit = {
                    amount: decrypted.amount,
                    token_type: decrypted.token_type,
                    token_address: decrypted.token_address,
                    token_id: decrypted.token_id,
                    is_rejected: true,
                    timestamp: null,
                };
            } else {
                deposit = {
                    amount: decrypted.amount,
                    token_type: decrypted.token_type,
                    token_address: decrypted.token_address,
                    token_id: decrypted.token_id,
                    is_rejected: false,
                    timestamp: metaData.timestamp,
                };
            }
        } else {
            deposit = {
                amount: decrypted.amount,
                token_type: decrypted.token_type,
                token_address: decrypted.token_address,
                token_id: decrypted.token_id,
                is_rejected: false,
                timestamp: null,
            };
        }
        history.push(deposit);
    }
    return history;
}


export interface Receive {
    amount: string;
    token_index: number;
    from: string,
    to: string,
    is_rejected: boolean;
    timestamp: number | null; // null if not yet confirmed
}

export async function fetchReceiveHistory(storeVaultClient: StoreVaultClient, privateKey: string, userData: JsUserData): Promise<Receive[]> {
    const allData = await storeVaultClient.getAllAfter("transfer", userData.pubkey, 0);
    const processedUuids = userData.processed_transfer_uuids

    const history = [];
    for (const [metaData, data] of allData) {
        // decrypt data if possible 
        let decrypted: JsTransferData;
        try {
            decrypted = await decrypt_transfer_data(privateKey, data);
        } catch (error) {
            console.log("Error decrypting data: ", error);
            continue; // just ignore invalid data
        }

        let receive: Receive;
        if (BigInt(metaData.timestamp) <= userData.transfer_lpt) {
            if (!processedUuids.includes(metaData.uuid)) {
                receive = {
                    amount: decrypted.transfer.amount,
                    token_index: decrypted.transfer.token_index,
                    from: decrypted.sender,
                    to: decrypted.transfer.recipient.data,
                    is_rejected: true,
                    timestamp: null,
                };
            } else {
                receive = {
                    amount: decrypted.transfer.amount,
                    token_index: decrypted.transfer.token_index,
                    from: decrypted.sender,
                    to: decrypted.transfer.recipient.data,
                    is_rejected: false,
                    timestamp: metaData.timestamp,
                };
            }
        } else {
            receive = {
                amount: decrypted.transfer.amount,
                token_index: decrypted.transfer.token_index,
                from: decrypted.sender,
                to: decrypted.transfer.recipient.data,
                is_rejected: false,
                timestamp: null,
            }
        }
        history.push(receive);
    }
    return history;
}

export interface Send {
    transfers: Transfer[];
    is_rejected: boolean;
    timestamp: number | null; // null if not yet confirmed
}

export interface Transfer {
    amount: string;
    token_index: number;
    to: string,
    is_withdrawal: boolean;
}

export async function fetchSendHistory(storeVaultClient: StoreVaultClient, privateKey: string, userData: JsUserData): Promise<Send[]> {
    const allData = await storeVaultClient.getAllAfter("tx", userData.pubkey, 0);
    const processedUuids = userData.processed_tx_uuids

    const history = [];
    for (const [metaData, data] of allData) {
        // decrypt data if possible 
        let decrypted: JsTxData;
        try {
            decrypted = await decrypt_tx_data(privateKey, data);
        } catch (error) {
            console.log("Error decrypting data: ", error);
            continue; // just ignore invalid data
        }
        const transfers = decrypted.transfers
            .filter((transfer) => transfer.amount !== "0")
            .map((transfer) => {
                return {
                    amount: transfer.amount,
                    token_index: transfer.token_index,
                    to: transfer.recipient.data,
                    is_withdrawal: !transfer.recipient.is_pubkey,
                }
            });

        let send: Send;
        if (BigInt(metaData.timestamp) <= userData.tx_lpt) {
            if (!processedUuids.includes(metaData.uuid)) {
                send = {
                    transfers,
                    is_rejected: true,
                    timestamp: null,
                };
            } else {
                send = {
                    transfers,
                    is_rejected: false,
                    timestamp: metaData.timestamp,
                };
            }
        } else {
            send = {
                transfers,
                is_rejected: false,
                timestamp: null,
            }
        }
        history.push(send);
    }
    return history;
}

