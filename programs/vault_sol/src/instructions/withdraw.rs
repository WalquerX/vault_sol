use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    error::ErrorCode,
    state::Vault,
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump
    )]
    pub vault_state: Account<'info, Vault>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let user_key = self.user.key();
        let vault_bump = self.vault_state.vault_bump;

        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, user_key.as_ref(), &[vault_bump]]];

        let cpi_program = self.system_program.key();

        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(cpi_ctx, amount)
    }
}