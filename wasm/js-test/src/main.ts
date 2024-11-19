import { cleanEnv, url } from 'envalid';
import { Config, finalize_tx, generate_intmax_account_from_eth_key, get_user_data, JsGenericAddress, JsTransfer, mimic_deposit, prepare_deposit, query_proposal, send_tx_request, sync, sync_withdrawals, } from '../pkg';
import { constructBlock, postBlock, postEmptyBlock, syncValidityProof } from './state-manager';
import { generateRandomHex } from './utils';

const env = cleanEnv(process.env, {
  BASE_URL: url(),
});

async function main() {
  const baseUrl = env.BASE_URL;
  const config = Config.new(baseUrl, baseUrl, baseUrl, baseUrl, 7200n, 300n);

  // generate key
  const key = await generate_intmax_account_from_eth_key(generateRandomHex(32));
  const publicKey = key.pubkey;
  const privateKey = key.privkey;
  console.log("privateKey: ", privateKey);
  console.log("publicKey: ", publicKey);

  // deposit to the account
  const tokenIndex = 0; // 0 for ETH
  const amount = "123";
  const pubkeySaltHash = await prepare_deposit(config, privateKey, amount, tokenIndex);
  console.log("pubkeySaltHash: ", pubkeySaltHash);
  await mimic_deposit(baseUrl, pubkeySaltHash, tokenIndex, amount);

  await postEmptyBlock(baseUrl); // block builder post empty block (this is not used in production)
  await syncValidityProof(baseUrl); // block validity prover sync validity proof (this is not used in production)
  console.log("validity proof synced");

  // sync the account's balance proof 
  await sync(config, privateKey);
  console.log("balance proof synced");

  // get the account's balance
  let userData = await get_user_data(config, privateKey);
  let balances = userData.balances;
  for (let i = 0; i < balances.length; i++) {
    const balance = balances[i];
    console.log(`Token ${balance.token_index}: ${balance.amount}`);
  }

  // construct a transfer tx
  const someonesKey = await generate_intmax_account_from_eth_key(generateRandomHex(32));
  const genericAddress = new JsGenericAddress(true, someonesKey.pubkey);
  const salt = generateRandomHex(32);
  const transfer = new JsTransfer(genericAddress, 0, "1", salt);

  // send the tx request
  const memo = await send_tx_request(config, baseUrl, privateKey, [transfer]
  );
  const tx = memo.tx();
  console.log("tx.nonce", tx.nonce);
  console.log("tx.transfer_tree_root", tx.transfer_tree_root);

  await constructBlock(baseUrl); // block builder construct block (this is not used in production)

  // query the block proposal
  const proposal = await query_proposal(config, baseUrl, privateKey, tx);
  if (proposal === undefined) {
    throw new Error("No proposal found");
  }
  // finalize the tx
  await finalize_tx(config, baseUrl, privateKey, memo, proposal);

  await postBlock(baseUrl); // block builder post block (this is not used in production)
  await syncValidityProof(baseUrl); // block validity prover sync validity proof (this is not used in production)
  console.log("validity proof synced");

  // get the receiver's balance
  await sync(config, someonesKey.privkey);
  console.log("balance proof synced");
  userData = await get_user_data(config, someonesKey.privkey);
  balances = userData.balances;
  for (let i = 0; i < balances.length; i++) {
    const balance = balances[i];
    console.log(`Token ${balance.token_index}: ${balance.amount}`);
  }

  // Withdrawal 
  const withdrawalEthAddress = generateRandomHex(20);
  const withdrawalTokenIndex = 0;
  const withdrawalAmount = "1";
  const withdrawalSalt = generateRandomHex(32);
  const withdrawalTransfer = new JsTransfer(new JsGenericAddress(false, withdrawalEthAddress), withdrawalTokenIndex, withdrawalAmount, withdrawalSalt);

  const withdrawalMemo = await send_tx_request(config, baseUrl, privateKey, [withdrawalTransfer]);
  console.log("withdrawalMemo.tx().nonce", withdrawalMemo.tx().nonce);
  console.log("withdrawalMemo.tx().transfer_tree_root", withdrawalMemo.tx().transfer_tree_root);

  await constructBlock(baseUrl); // block builder construct block (this is not used in production)

  const proposal2 = await query_proposal(config, baseUrl, privateKey, withdrawalMemo.tx());
  if (proposal2 === undefined) {
    throw new Error("No proposal found");
  }
  await finalize_tx(config, baseUrl, privateKey, withdrawalMemo, proposal2);

  await postBlock(baseUrl); // block builder post block (this is not used in production)
  await syncValidityProof(baseUrl); // block validity prover sync validity proof (this is not used in production)
  console.log("validity proof synced");

  await new Promise((resolve) => setTimeout(resolve, 5000));

  // sync withdrawals 
  await sync_withdrawals(config, privateKey);
  console.log("Withdrawal synced");
}

main().then(() => {
  process.exit(0);
}).catch((err) => {
  console.error(err);
  process.exit(1);
});