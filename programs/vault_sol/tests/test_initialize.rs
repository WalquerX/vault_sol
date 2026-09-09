use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{
            instruction::{AccountMeta, Instruction},
            system_program,
        },
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    vault_sol::constants::{STATE_SEED, VAULT_SEED},
};

const ONE_SOL: u64 = 1_000_000_000;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct Ctx {
    svm: LiteSVM,
    payer: Keypair,
    program_id: Pubkey,
    vault_state: Pubkey,
    vault: Pubkey,
}

impl Ctx {
    fn new() -> Self {
        let program_id = vault_sol::id();
        let payer = Keypair::new();

        let (vault_state, _) =
            Pubkey::find_program_address(&[STATE_SEED, payer.pubkey().as_ref()], &program_id);
        let (vault, _) =
            Pubkey::find_program_address(&[VAULT_SEED, payer.pubkey().as_ref()], &program_id);

        let mut svm = LiteSVM::new();
        let bytes = include_bytes!(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/../deploy/vault_sol.so"
        ));
        svm.add_program(program_id, bytes).unwrap();
        svm.airdrop(&payer.pubkey(), 10 * ONE_SOL).unwrap();

        Self {
            svm,
            payer,
            program_id,
            vault_state,
            vault,
        }
    }

    fn send(&mut self, data: Vec<u8>, metas: Vec<AccountMeta>) -> TransactionResult {
        let ix = Instruction::new_with_bytes(self.program_id, &data, metas);
        let blockhash = self.svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[ix], Some(&self.payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&self.payer]).unwrap();
        self.svm.send_transaction(tx)
    }

    // --- instructions ---

    fn initialize(&mut self) -> TransactionResult {
        let metas = vault_sol::accounts::Initialize {
            user: self.payer.pubkey(),
            vault_state: self.vault_state,
            vault: self.vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None);
        self.send(vault_sol::instruction::Initialize {}.data(), metas)
    }

    fn deposit(&mut self, amount: u64) -> TransactionResult {
        let metas = vault_sol::accounts::Deposit {
            user: self.payer.pubkey(),
            vault_state: self.vault_state,
            vault: self.vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None);
        self.send(vault_sol::instruction::Deposit { amount }.data(), metas)
    }

    fn withdraw(&mut self, amount: u64) -> TransactionResult {
        let metas = vault_sol::accounts::Withdraw {
            user: self.payer.pubkey(),
            vault_state: self.vault_state,
            vault: self.vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None);
        self.send(vault_sol::instruction::Withdraw { amount }.data(), metas)
    }

    fn close(&mut self) -> TransactionResult {
        let metas = vault_sol::accounts::Close {
            user: self.payer.pubkey(),
            vault_state: self.vault_state,
            vault: self.vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None);
        self.send(vault_sol::instruction::Close {}.data(), metas)
    }

    // --- reads ---

    fn state(&self) -> vault_sol::state::Vault {
        let account = self.svm.get_account(&self.vault_state).unwrap();
        let mut data: &[u8] = &account.data;
        vault_sol::state::Vault::try_deserialize(&mut data).unwrap()
    }

    fn vault_lamports(&self) -> u64 {
        self.svm
            .get_account(&self.vault)
            .map(|a| a.lamports)
            .unwrap_or(0)
    }

    fn user_lamports(&self) -> u64 {
        self.svm.get_account(&self.payer.pubkey()).unwrap().lamports
    }

    fn expected_bumps(&self) -> (u8, u8) {
        let (_, state_bump) =
            Pubkey::find_program_address(&[STATE_SEED, self.payer.pubkey().as_ref()], &self.program_id);
        let (_, vault_bump) =
            Pubkey::find_program_address(&[VAULT_SEED, self.payer.pubkey().as_ref()], &self.program_id);
        (state_bump, vault_bump)
    }
}

// ---------------------------------------------------------------------------
// Happy paths
// ---------------------------------------------------------------------------

#[test]
fn initialize_stores_both_bumps() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();

    let (state_bump, vault_bump) = ctx.expected_bumps();
    let state = ctx.state();

    assert_eq!(state.state_bump, state_bump);
    assert_eq!(state.vault_bump, vault_bump);
    let expected = ctx.svm.minimum_balance_for_rent_exemption(0);
    assert_eq!(ctx.vault_lamports(), expected);
}

#[test]
fn deposit_increases_vault_balance() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();

    let before = ctx.vault_lamports();
    ctx.deposit(ONE_SOL).unwrap();

    assert_eq!(ctx.vault_lamports(), before + ONE_SOL);
}

#[test]
fn withdraw_decreases_vault_balance() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();
    ctx.deposit(2 * ONE_SOL).unwrap();

    let before = ctx.vault_lamports();
    ctx.withdraw(ONE_SOL).unwrap();

    assert_eq!(ctx.vault_lamports(), before - ONE_SOL);
}

#[test]
fn close_empties_the_vault() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();
    ctx.deposit(ONE_SOL).unwrap();

    let user_before = ctx.user_lamports();
    ctx.close().unwrap();

    assert_eq!(ctx.vault_lamports(), 0);
    assert!(ctx.user_lamports() > user_before, "user must get lamports back");
    assert!(ctx.svm.get_account(&ctx.vault_state).is_none(), "state account must be closed");
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn deposit_rejects_zero_amount() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();

    assert!(ctx.deposit(0).is_err());
}

#[test]
fn withdraw_rejects_zero_amount() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();
    ctx.deposit(ONE_SOL).unwrap();

    assert!(ctx.withdraw(0).is_err());
}

#[test]
fn deposit_fails_before_initialize() {
    let mut ctx = Ctx::new();

    assert!(ctx.deposit(ONE_SOL).is_err());
}

#[test]
fn withdraw_fails_when_amount_exceeds_balance() {
    let mut ctx = Ctx::new();
    ctx.initialize().unwrap();
    ctx.deposit(ONE_SOL).unwrap();

    let balance = ctx.vault_lamports();
    assert!(ctx.withdraw(balance + ONE_SOL).is_err());
}