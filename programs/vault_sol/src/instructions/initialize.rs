use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{constants::*, state::Vault};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = Vault::DISCRIMINATOR.len() + Vault::INIT_SPACE,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump
    )]
    pub vault_state: Account<'info, Vault>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.data_len());

        let cpi_program = self.system_program.key();

        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, rent_exempt)?;

        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;

        Ok(())
    }
}
