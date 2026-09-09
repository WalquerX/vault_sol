use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Amount not allowed on this tx")]
    InvalidAmount,
}