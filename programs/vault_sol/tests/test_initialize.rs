
use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_initialize() {
    let program_id = vault_sol::id();
    let payer = Keypair::new();

    let (vault_state, _) = Pubkey::find_program_address(
        &[vault_sol::constants::STATE_SEED, payer.pubkey().as_ref()],
        &program_id,
    );
    let (vault, _) = Pubkey::find_program_address(
        &[vault_sol::constants::VAULT_SEED, payer.pubkey().as_ref()],
        &program_id,
    );

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/vault_sol.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &vault_sol::instruction::Initialize {}.data(),
        vault_sol::accounts::Initialize {
            user: payer.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());

    // state account holds the two bumps
    let state_account = svm.get_account(&vault_state).unwrap();
    let mut data: &[u8] = &state_account.data;
    let state = vault_sol::state::Vault::try_deserialize(&mut data).unwrap();

    let (_, expected_state_bump) = Pubkey::find_program_address(
        &[vault_sol::constants::STATE_SEED, payer.pubkey().as_ref()],
        &program_id,
    );
    let (_, expected_vault_bump) = Pubkey::find_program_address(
        &[vault_sol::constants::VAULT_SEED, payer.pubkey().as_ref()],
        &program_id,
    );
    assert_eq!(state.state_bump, expected_state_bump);
    assert_eq!(state.vault_bump, expected_vault_bump);

    // vault got the rent-exempt minimum for a zero-data account
    let vault_account = svm.get_account(&vault).unwrap();
    assert!(vault_account.lamports > 0);
}
