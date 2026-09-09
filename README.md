# vault_sol

A SOL vault built with Anchor. Each user gets one personal vault. Only
the user can deposit to it, withdraw from it, or close it.

## Accounts

The program derives two PDAs per user.

| Account | Seeds | Owner | Purpose |
|---|---|---|---|
| `vault_state` | `[b"state", user]` | this program | Stores the two bumps |
| `vault` | `[b"vault", user]` | System Program | Holds the lamports |

Two accounts are necessary. Only the System Program can move lamports out of an
account through a CPI, so the vault must stay system-owned. An account that
stores program data must be owned by the program. One account cannot do both.

## Instructions

| Instruction | Action |
|---|---|
| `initialize` | Creates `vault_state`, funds `vault` to the rent-exempt minimum, stores both bumps. |
| `deposit(amount)` | Moves `amount` lamports from the user to the vault. |
| `withdraw(amount)` | Moves `amount` lamports from the vault to the user, signed by the vault PDA. |
| `close` | Drains the vault and closes `vault_state`, returning all rent to the user. |

## Design notes

**Stored bumps.** `initialize` writes both bumps into `vault_state`. The other
instructions read them with `bump = vault_state.vault_bump` instead of a call to
`find_program_address`. This removes the off-curve search from each call.

**PDA signing.** `withdraw` and `close` send lamports from the vault, which has
no private key. They use `CpiContext::new_with_signer` with the vault seeds and
the stored bump.

**Amount checks.** Each transfer instruction rejects a zero amount. No upper
bound check is necessary. The System Program refuses a transfer that leaves the
source below its rent-exempt minimum.