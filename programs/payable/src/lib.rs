use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("8rrFrtdFK3x8NBFEvaqznHg9Q9Tf2ij6bHEv2FCHotgJ");

#[program]
pub mod payable {
    use anchor_spl::token::{transfer, Transfer};

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.payable_idx_counter = 0;

        emit!(Initialized {});
        Ok(())
    }

    pub fn create_payable(
        ctx: Context<CreatePayable>,
        amount: i64,
        recurrent: bool,
        number_of_recurrent_payment: i64,
        recurrent_payment_interval: i64,
        cancel_period: i64,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let payable = &mut ctx.accounts.payable;
        let clock = Clock::get()?;

        // check creator is not payer
        require_keys_neq!(ctx.accounts.signer.key(), ctx.accounts.payer.key());

        let payable_idx = counter.payable_idx_counter;
        let creator = ctx.accounts.signer.key();
        let valid_token = ctx.accounts.valid_token_mint.key();
        let payer = ctx.accounts.payer.key();

        // create payable
        payable.payable_idx = payable_idx;
        payable.amount = amount;
        payable.cancel_period = clock.unix_timestamp + cancel_period;
        payable.creator = ctx.accounts.signer.key();
        payable.payer = payer;
        payable.recurrent = recurrent;
        payable.number_of_recurrent_payment = number_of_recurrent_payment;
        payable.recurrent_payment_interval = recurrent_payment_interval;
        payable.valid_payment_token = ctx.accounts.valid_token_mint.key();
        payable.last_withdrawal = clock.unix_timestamp;
        payable.status = 0; // locked

        // update counter
        counter.payable_idx_counter += 1;

        // emit event
        emit!(PayableCreated {
            payable_idx,
            creator,
            payer,
            valid_token,
            amount
        });

        Ok(())
    }

    pub fn accept_payable(ctx: Context<AcceptPayable>, recurrent: bool) -> Result<()> {
        let payable = &mut ctx.accounts.payable;
        let amount = payable
            .amount
            .checked_mul(payable.number_of_recurrent_payment)
            .expect("Payable::Functions::AcceptPayable::Overflow Error");

        // user must be aware of recurrent payment
        require_eq!(payable.recurrent, recurrent);

        // only valid payer can accept payable
        require_keys_eq!(payable.payer, ctx.accounts.signer.key());

        // update payable
        payable.status = 1;

        // lock token to cover all payment
        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_ata.to_account_info(),
            to: ctx.accounts.payable_ata.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        transfer(CpiContext::new(cpi_program, cpi_accounts), amount as u64)?;

        // emit event
        emit!(PayableAccepted {
            payable_idx: payable.payable_idx,
            creator: payable.creator,
            payer: payable.payer,
            valid_token: payable.valid_payment_token,
            amount
        });

        Ok(())
    }

    pub fn cancel_payable(ctx: Context<CancelPayable>) -> Result<()> {
        let clock = Clock::get()?;

        // only valid payer can cancel payable
        require_keys_eq!(ctx.accounts.payable.payer, ctx.accounts.signer.key());

        // get signer seed
        let bump = ctx.bumps.payable;
        let payee_seed = ctx.accounts.payee.key();
        let payer_seed = ctx.accounts.signer.key();

        let seeds = &[
            &b"payable"[..],
            payee_seed.as_ref(),
            payer_seed.as_ref(),
            &[bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // // withdraw outstanding payment
        // get number of missed payment
        let mut missed_payment_withdrawal = 1;

        if ctx.accounts.payable.number_of_recurrent_payment > 0 {
            let last_withdrawal_period = clock
                .unix_timestamp
                .checked_sub(ctx.accounts.payable.last_withdrawal)
                .expect("Payable::Functions::CancelPayable::Overflow Error");
            missed_payment_withdrawal = last_withdrawal_period
                .checked_div(ctx.accounts.payable.recurrent_payment_interval)
                .expect("Payable::Functions::CancelPayable::Overflow Error");
        }

        // get amount to transfer
        let amount_to_transfer = missed_payment_withdrawal * ctx.accounts.payable.amount as i64;
        ctx.accounts.payable.number_of_recurrent_payment = ctx
            .accounts
            .payable
            .number_of_recurrent_payment
            .checked_sub(missed_payment_withdrawal)
            .expect("Payable::Functions::CancelPayable::Overflow Error");

        // transfer amount_to_transfer to payee
        let cpi_accounts = Transfer {
            from: ctx.accounts.payable_ata.to_account_info(),
            to: ctx.accounts.payee_ata.to_account_info(),
            authority: ctx.accounts.payable.to_account_info(),
        };
        let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
        transfer(
            CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
            amount_to_transfer as u64,
        )?;

        // check if cancel period is over
        let cancel_period_is_over = clock.unix_timestamp >= ctx.accounts.payable.cancel_period;

        // if cancel period is over, transfer 50% of single payable payment to payee and send the rest to payer
        if cancel_period_is_over {
            // check if payable is recurrent
            if ctx.accounts.payable.number_of_recurrent_payment > 0 {
                let number_of_recurrent_payment_left =
                    ctx.accounts.payable.number_of_recurrent_payment;
                let temp = number_of_recurrent_payment_left
                    .checked_mul(ctx.accounts.payable.amount)
                    .expect("Payable::Functions::CancelPayable::Overflow Error");
                let balance_left = temp
                    .checked_sub(ctx.accounts.payable.amount / 2)
                    .expect("Payable::Functions::CancelPayable::Overflow Error");

                // transfer balance_left to payer
                let cpi_accounts = Transfer {
                    from: ctx.accounts.payable_ata.to_account_info(),
                    to: ctx.accounts.payer_ata.to_account_info(),
                    authority: ctx.accounts.payable.to_account_info(),
                };
                let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
                transfer(
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                    balance_left as u64,
                )?;

                // transfer amount/2 to payee
                let cpi_accounts = Transfer {
                    from: ctx.accounts.payable_ata.to_account_info(),
                    to: ctx.accounts.payee_ata.to_account_info(),
                    authority: ctx.accounts.payable.to_account_info(),
                };
                let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
                transfer(
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                    ctx.accounts.payable.amount as u64 / 2,
                )?;
            } else {
                // transfer amount/2 to payer
                let cpi_accounts = Transfer {
                    from: ctx.accounts.payable_ata.to_account_info(),
                    to: ctx.accounts.payer_ata.to_account_info(),
                    authority: ctx.accounts.payable.to_account_info(),
                };
                let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
                transfer(
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                    ctx.accounts.payable.amount as u64 / 2,
                )?;

                // transfer amount/2 to payee
                let cpi_accounts = Transfer {
                    from: ctx.accounts.payable_ata.to_account_info(),
                    to: ctx.accounts.payee_ata.to_account_info(),
                    authority: ctx.accounts.payable.to_account_info(),
                };
                let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
                transfer(
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                    ctx.accounts.payable.amount as u64 / 2,
                )?;
            }
        } else {
            // update payable status
            ctx.accounts.payable.status = 0;

            // if cancel period is not over, transfer 100% to payer
            if ctx.accounts.payable.number_of_recurrent_payment > 0 {
                let number_of_recurrent_payment_left =
                    ctx.accounts.payable.number_of_recurrent_payment;
                let balance_left = number_of_recurrent_payment_left
                    .checked_mul(ctx.accounts.payable.amount)
                    .expect("Payable::Functions::CancelPayable::Overflow Error");

                // transfer balance_left to payer
                let cpi_accounts = Transfer {
                    from: ctx.accounts.payable_ata.to_account_info(),
                    to: ctx.accounts.payer_ata.to_account_info(),
                    authority: ctx.accounts.payable.to_account_info(),
                };
                let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
                transfer(
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                    balance_left as u64,
                )?;
            } else {
                // transfer amount to payer
                let cpi_accounts = Transfer {
                    from: ctx.accounts.payable_ata.to_account_info(),
                    to: ctx.accounts.payer_ata.to_account_info(),
                    authority: ctx.accounts.payable.to_account_info(),
                };
                let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
                transfer(
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                    ctx.accounts.payable.amount as u64,
                )?;
            }

        }

        Ok(())
    }

    pub fn withdraw(ctx: Context<WithdrawFromPayable>) -> Result<()> {
        let clock = Clock::get()?;

        // only creator can withdraw
        require_keys_eq!(ctx.accounts.payable.creator, ctx.accounts.signer.key());

        // withdraw can happen after cancel period
        require!(
            clock.unix_timestamp >= ctx.accounts.payable.cancel_period,
            Error::WithdrawalTimeNotReached
        );

        // payable must have been accepted
        require_eq!(ctx.accounts.payable.status, 1);

        // must have pending withdrawal
        require!(ctx.accounts.payable.number_of_recurrent_payment > 0, Error::CompletedPayble);

        // get signer seed
        let bump = ctx.bumps.payable;
        let payee_seed = ctx.accounts.signer.key();
        let payer_seed = ctx.accounts.payer.key();

        let seeds = &[
            &b"payable"[..],
            payee_seed.as_ref(),
            payer_seed.as_ref(),
            &[bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // if payment is recurrent, check how many payment is left to be withdrawn
        // transfer amount * number of payment not withdrawn
        if ctx.accounts.payable.number_of_recurrent_payment > 1 {
            let last_withdrawal_period = clock.unix_timestamp.checked_sub(ctx.accounts.payable.last_withdrawal).expect("msg");
            let missed_payment_withdrawal = last_withdrawal_period.checked_div(ctx.accounts.payable.recurrent_payment_interval).expect("msg");
            
            let amount_to_transfer = missed_payment_withdrawal * ctx.accounts.payable.amount;

            // update payment remaining
            ctx.accounts.payable.number_of_recurrent_payment = ctx.accounts.payable.number_of_recurrent_payment.checked_sub(missed_payment_withdrawal).expect("msg");
            
            // transfer amount_to_transfer to payee
            let cpi_accounts = Transfer {
                from: ctx.accounts.payable_ata.to_account_info(),
                to: ctx.accounts.payee_ata.to_account_info(),
                authority: ctx.accounts.payable.to_account_info(),
            };
            let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
            transfer(
                CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                amount_to_transfer as u64,
            )?;
        } else {
            // update payment remaining
            ctx.accounts.payable.number_of_recurrent_payment = ctx.accounts.payable.number_of_recurrent_payment - 1;

            // update status 
            ctx.accounts.payable.status = 2;

            // transfer amount to payee
            let cpi_accounts = Transfer {
                from: ctx.accounts.payable_ata.to_account_info(),
                to: ctx.accounts.payee_ata.to_account_info(),
                authority: ctx.accounts.payable.to_account_info(),
            };
            let cpi_program = ctx.accounts.valid_token_mint.to_account_info();
            transfer(
                CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds),
                ctx.accounts.payable.amount as u64,
            )?;

            emit!(PayableCompleted{
                payable_idx: ctx.accounts.payable.payable_idx,
                creator: ctx.accounts.payable.creator,
                payer: ctx.accounts.payer.key(),
                valid_token: ctx.accounts.payable.valid_payment_token,
            })
        }

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
pub struct CreatePayable<'info> {
    #[account(mut, seeds = [b"counter"], bump)]
    pub counter: Account<'info, Counter>,
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + Payable::LEN,
        seeds = [
            b"payable",
            signer.key().as_ref(),
            payer.key().as_ref()
        ],
        bump
    )]
    pub payable: Account<'info, Payable>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub payer: AccountInfo<'info>,

    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptPayable<'info> {
    #[account(
        mut,
        seeds = [
            b"payable", 
            payee.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub payable: Account<'info, Payable>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub payee: AccountInfo<'info>,

    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    // payer valid token ATA
    #[account(mut)]
    pub payer_ata: Account<'info, TokenAccount>,

    // vault valid token ATA
    #[account(mut)]
    pub payable_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelPayable<'info> {
    #[account(
        mut,
        seeds = [
            b"payable", 
            payee.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub payable: Account<'info, Payable>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub payee: AccountInfo<'info>,

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
    pub payable_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawFromPayable<'info> {
    #[account(
        mut,
        seeds = [
            b"payable",
            signer.key().as_ref(),
            payer.key().as_ref(),
        ],
        bump
    )]
    pub payable: Account<'info, Payable>,
    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub payer: AccountInfo<'info>,

    #[account(mut)]
    pub valid_token_mint: Account<'info, Mint>,

    // payer valid token ATA
    #[account(mut)]
    pub payee_ata: Account<'info, TokenAccount>,

    // vault valid token ATA
    #[account(mut)]
    pub payable_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    // account holding the contract binary
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Counter {
    pub payable_idx_counter: u64,
}

#[account]
pub struct Payable {
    pub payable_idx: u64,
    pub amount: i64,
    pub cancel_period: i64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub recurrent: bool,
    pub number_of_recurrent_payment: i64,
    pub recurrent_payment_interval: i64,
    pub valid_payment_token: Pubkey,
    pub last_withdrawal: i64,
    pub status: u8, // 0 = Created, 1 = Accepted, 2 = Completed
}

impl Counter {
    pub const LEN: usize = (1 + 16);
}

impl Payable {
    pub const LEN: usize = (1 + 8)  +          // payable_idx
        (1 + 8)  +          // amount
        (1 + 8)  +          // critical period
        (1 + 32)  +         // creator
        (1 + 32) +          // payer
        (1 + 1)  +          // recurrent
        (1 + 8)  +          // number of recurrent payment
        (1 + 8)  +          // recurrent payment interval
        (1 + 32)  +         // valid token
        (1 + 8)  +          // last withdrawal
        (1 + 1); // status
}

#[event]
pub struct Initialized {}

#[event]
pub struct PayableCreated {
    pub payable_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
    pub amount: i64,
}

#[event]
pub struct PayableAccepted {
    pub payable_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
    pub amount: i64,
}

#[event]
pub struct CancelPeriodOver {
    pub payable_idx: u64,
}

#[event]
pub struct PayableWithdrawal {
    pub payable_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
    pub amount: i64,
}

#[event]
pub struct PayableCompleted {
    pub payable_idx: u64,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub valid_token: Pubkey,
}

#[error_code]
pub enum Error {
    CyclicPayable,
    WithdrawalTimeNotReached,
    CompletedPayble
}
