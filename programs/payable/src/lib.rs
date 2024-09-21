use anchor_lang::prelude::*;

declare_id!("8rrFrtdFK3x8NBFEvaqznHg9Q9Tf2ij6bHEv2FCHotgJ");

#[program]
pub mod payable {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        emit!(Initialized {});

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}


#[account]
pub struct Invoice {
    pub invoice_idx: u128,
    pub amount: u128,
    pub critical_period: u128,
    pub creator: Pubkey,
    pub payers: Vec<Pubkey>, // max 5 payers on a single invoice
    pub recurrent: bool,
    pub number_of_recurrent_payment: u128,
    pub recurrent_payment_interval: u128,
    pub valid_payment_token: Pubkey,
    pub last_withdrawal: u128,
    pub status: u8, // 0 = locked, 1 = unlocked, 2 = completed
}

impl Invoice {
    pub const LEN: usize =
        (1 + 16)  +         // invoice_idx
        (1 + 16)  +         // amount
        (1 + 16)  +         // critical period
        (1 + 32)  +         // creator
        (1 + (5 * 32)) +    // payers
        (1 + 1)  +          // recurrent
        (1 + 16)  +         // number of recurrent payment
        (1 + 16)  +         // recurrent payment interval
        (1 + 32)  +         // valid token
        (1 + 16)  +         // last withdrawal
        (1 + 1);            // critical period 
}

#[event]
pub struct Initialized {}