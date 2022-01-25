use anchor_client::solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

pub(crate) fn get_or_create_ata(
    rpc: &RpcClient,
    wallet: Pubkey,
    mint: Pubkey,
    payer: &Keypair,
) -> Pubkey {
    let ata = spl_associated_token_account::get_associated_token_address(&wallet, &mint);
    if rpc.get_account_data(&ata).is_err() {
        let hash = rpc.get_latest_blockhash().unwrap();
        let create_token_acc_tx = Transaction::new_signed_with_payer(
            &[
                spl_associated_token_account::create_associated_token_account(
                    &payer.pubkey(),
                    &wallet,
                    &mint,
                ),
            ],
            Some(&payer.pubkey()),
            &[payer],
            hash,
        );
        let sigs = rpc
            .send_and_confirm_transaction(&create_token_acc_tx)
            .unwrap();
        println!("\n Newly create SIG: {} => {}", sigs, ata);
    } else {
        println!("\n>>> Already have {}", ata);
    }

    ata
}
