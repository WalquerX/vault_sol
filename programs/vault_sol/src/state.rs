use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Vault { // we store the bumps to dont calculate them each time
    pub vault_bump: u8,
    pub state_bump: u8,
}
