use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("8rrFrtdFK3x8NBFEvaqznHg9Q9Tf2ij6bHEv2FCHotgJ");

#[program]
pub mod payable {
    use anchor_spl::token::{transfer, Transfer};

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.invoice_idx_counter = 0;

        emit!(Initialized {});
        Ok(())
    }

    pub fn create_invoice(
        ctx: Context<CreateInvoice>, 
        amount: u64, 
        recurrent: bool, 
        number_of_recurrent_payment: u64, 
        recurrent_payment_interval: u64,
        cancel_period: i64
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let invoice = &mut ctx.accounts.invoice;
        let clock = Clock::get()?;

        // check creator is not payer
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer.key());

        let invoice_idx = counter.invoice_idx_counter;
        let creator = ctx.accounts.signer.key();
        let valid_token = ctx.accounts.valid_token_mint.key();
        let payer = ctx.accounts.payer.key();

        // create invoice 
        invoice.invoice_idx = invoice_idx;
        invoice.amount = amount;
        invoice.cancel_period = clock.unix_timestamp + cancel_period;
        invoice.creator = ctx.accounts.signer.key();
        invoice.payer = payer;
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
            payer,
            valid_token,
            amount
        });

        Ok(())
    }

    pub fn accept_invoice(
        ctx: Context<AcceptInvoice>, 
        recurrent: bool
    ) -> Result<()> {
        let invoice = &mut ctx.accounts.invoice;
        let amount = invoice.amount.checked_mul(invoice.number_of_recurrent_payment).expect(
            "Payable::Functions::AcceptInvoice::Overflow Error");

        // user must be aware of recurrent payment
        require_eq!(invoice.recurrent, recurrent);

        // only valid payer can accept invoice
        require_keys_eq!(invoice.payer, ctx.accounts.signer.key());

        // update invoice
        invoice.status = 1;

        // lock token to cover all payment
        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_ata.to_account_info(),
            to: ctx.accounts.invoice_ata.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        transfer(CpiContext::new(
            cpi_program, cpi_accounts), 
            amount 
        )?;

        // emit event 
        emit!(InvoiceAccepted {
            invoice_idx: invoice.invoice_idx,
            creator: invoice.creator,
            payer: invoice.payer,
            valid_token: invoice.valid_payment_token,
            amount
        });
        
        Ok(())
    }

    pub fn cancel_invoice(
        _ctx: Context<CancelInvoice>, 
    ) -> Result<()> {
        // let invoice = &mut ctx.accounts.invoice;
        // let clock = Clock::get()?;

        // check cancel period

        // if cancel period is over, transfer 50% of single invoice payment to payee and send the rest to payer

        // if cancel period is over, transfer 100% to payer

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
            payer.key().as_ref()
        ], 
        bump
    )]
    pub invoice: Account<'info, Invoice>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    /// must provide payer one
    #[account(mut)]
    pub payer: AccountInfo<'info>,
    
    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptInvoice<'info> {
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + Invoice::LEN,
        seeds = [
            b"invoice", 
            payee.key().as_ref(), 
            signer.key().as_ref()
        ], 
        bump
    )]
    pub invoice: Account<'info, Invoice>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    /// must provide payer one
    #[account(mut)]
    pub payee: AccountInfo<'info>,

    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    // payer valid token ATA 
    #[account(mut)]
    pub payer_ata: Account<'info, TokenAccount>,

    // vault valid token ATA 
    #[account(mut)]
    pub invoice_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelInvoice<'info> {
    #[account(
        mut,
        seeds = [
            b"invoice", 
            signer.key().as_ref(), 
            payer.key().as_ref(),
        ], 
        bump
    )]
    pub invoice: Account<'info, Invoice>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    /// must provide payer one
    #[account(mut)]
    pub payer: AccountInfo<'info>,
    
    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    // payer valid token ATA 
    #[account(mut)]
    pub payee_ata: Account<'info, TokenAccount>,

    // payer valid token ATA 
    #[account(mut)]
    pub payer_ata: Account<'info, TokenAccount>,

    // payer valid token ATA 
    #[account(mut)]
    pub invoice_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Counter {
    pub invoice_idx_counter: u64,
}

#[account]
pub struct Invoice {
    pub invoice_idx: u64,
    pub amount: u64,
    pub cancel_period: i64,
    pub creator: Pubkey,
    pub payer: Pubkey, 
    pub recurrent: bool,
    pub number_of_recurrent_payment: u64,
    pub recurrent_payment_interval: u64,
    pub valid_payment_token: Pubkey,
    pub last_withdrawal: i64,
    pub status: u8, // 0 = Created, 1 = Accepted, 2 = Completed
}

impl Counter {
    pub const LEN: usize =
        (1 + 16);
}

impl Invoice {
    pub const LEN: usize =
        (1 + 8)  +         // invoice_idx
        (1 + 8)  +          // amount
        (1 + 8)  +          // critical period
        (1 + 32)  +         // creator
        (1 + 32) +          // payer
        (1 + 1)  +          // recurrent
        (1 + 8)  +          // number of recurrent payment
        (1 + 8)  +         // recurrent payment interval
        (1 + 32)  +         // valid token
        (1 + 8)  +          // last withdrawal
        (1 + 1);            // status
}

#[event]
pub struct Initialized {}

#[event]
pub struct InvoiceCreated {
    pub invoice_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
    pub amount: u64
}

#[event]
pub struct InvoiceAccepted {
    pub invoice_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
    pub amount: u64
}

#[event]
pub struct CancelPeriodOver {
    pub invoice_idx: u64
}

#[event]
pub struct InvoiceWithdrawal {
    pub invoice_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
    pub amount: u64
}

#[event]
pub struct InvoiceCompleted {
    pub invoice_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
}

#[error_code]
pub enum Error {
    CyclicInvoice
}