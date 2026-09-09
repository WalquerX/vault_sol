use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Only the authority can update")]
    Unauthorized,
}