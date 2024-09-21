use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

declare_id!("8rrFrtdFK3x8NBFEvaqznHg9Q9Tf2ij6bHEv2FCHotgJ");

#[program]
pub mod payable {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.invoice_idx_counter = 0;

        emit!(Initialized {});
        Ok(())
    }

    pub fn create_invoice(
        ctx: Context<CreateInvoice>, 
        amount: u128, 
        recurrent: bool, 
        number_of_recurrent_payment: u128, 
        recurrent_payment_interval: u128,
        cancel_period: i64
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let invoice = &mut ctx.accounts.invoice;
        let clock = Clock::get()?;

        // check creator is not payer
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer_1.key());
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer_2.key());
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer_3.key());
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer_3.key());
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer_4.key());
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer_5.key());
        
        // create payers vec
        let mut payers = vec![];
        payers.push(ctx.accounts.payer_1.key());
        payers.push(ctx.accounts.payer_2.key());
        payers.push(ctx.accounts.payer_3.key());
        payers.push(ctx.accounts.payer_4.key());
        payers.push(ctx.accounts.payer_5.key());

        let invoice_idx = counter.invoice_idx_counter;
        let creator = ctx.accounts.signer.key();
        let valid_token = ctx.accounts.valid_token_mint.key();

        // create invoice 
        invoice.invoice_idx = invoice_idx;
        invoice.amount = amount;
        invoice.cancel_period = clock.unix_timestamp + cancel_period;
        invoice.creator = ctx.accounts.signer.key();
        invoice.payers.append(&mut payers);
        invoice.recurrent = recurrent;
        invoice.number_of_recurrent_payment = number_of_recurrent_payment;
        invoice.recurrent_payment_interval = recurrent_payment_interval;
        invoice.valid_payment_token = ctx.accounts.valid_token_mint.key();
        invoice.last_withdrawal = clock.unix_timestamp;
        invoice.status = 0; // locked

        // update counter
        counter.invoice_idx_counter += 1;

        // emit event
        emit!(InvoiceCreated {
            invoice_idx,
            creator,
            payers,
            valid_token,
            amount
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + Counter::LEN,
        seeds = [b"counter"], 
        bump
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub signer: Signer<'info>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateInvoice<'info> {
    #[account(mut, seeds = [b"counter"], bump)]
    pub counter: Account<'info, Counter>,
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + Invoice::LEN,
        seeds = [
            b"invoice", 
            signer.key().as_ref(), 
            payer_1.key().as_ref(), 
            payer_2.key().as_ref(), 
            payer_3.key().as_ref(), 
            payer_4.key().as_ref(), 
            payer_5.key().as_ref()
        ], 
        bump
    )]
    pub invoice: Account<'info, Invoice>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    /// must provide payer one
    #[account(mut)]
    pub payer_1: AccountInfo<'info>,
    
    // use random address as default value
    /// CHECK: safe
    #[account(mut)]
    pub payer_2: AccountInfo<'info>,
    /// CHECK: safe
    #[account(mut)]
    pub payer_3: AccountInfo<'info>,
    /// CHECK: safe
    #[account(mut)]
    pub payer_4: AccountInfo<'info>,
    /// CHECK: safe
    #[account(mut)]
    pub payer_5: AccountInfo<'info>,

    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Counter {
    pub invoice_idx_counter: u128,
}

#[account]
pub struct Invoice {
    pub invoice_idx: u128,
    pub amount: u128,
    pub cancel_period: i64,
    pub creator: Pubkey,
    pub payers: Vec<Pubkey>, // max 5 payers on a single invoice
    pub recurrent: bool,
    pub number_of_recurrent_payment: u128,
    pub recurrent_payment_interval: u128,
    pub valid_payment_token: Pubkey,
    pub last_withdrawal: i64,
    pub status: u8, // 0 = locked, 1 = unlocked, 2 = completed
}

impl Counter {
    pub const LEN: usize =
        (1 + 16);
}

impl Invoice {
    pub const LEN: usize =
        (1 + 16)  +         // invoice_idx
        (1 + 16)  +         // amount
        (1 + 8)  +          // critical period
        (1 + 32)  +         // creator
        (1 + (5 * 32)) +    // payers
        (1 + 1)  +          // recurrent
        (1 + 16)  +         // number of recurrent payment
        (1 + 16)  +         // recurrent payment interval
        (1 + 32)  +         // valid token
        (1 + 8)  +         // last withdrawal
        (1 + 1);            // critical period 
}

#[event]
pub struct Initialized {}

#[event]
pub struct InvoiceCreated {
    pub invoice_idx: u128,
    pub creator: Pubkey,
    pub payers: Vec<Pubkey>,
    pub valid_token: Pubkey,
    pub amount: u128
}

#[event]
pub struct CancelPeriodOver {
    pub invoice_idx: u128
}

#[event]
pub struct InvoiceWithdrawal {
    pub invoice_idx: u128,
    pub creator: Pubkey,
    pub payers: Vec<Pubkey>,
    pub valid_token: Pubkey,
    pub amount: u128
}

#[event]
pub struct InvoiceCompleted {
    pub invoice_idx: u128,
    pub creator: Pubkey,
    pub payers: Vec<Pubkey>,
    pub valid_token: Pubkey,
}