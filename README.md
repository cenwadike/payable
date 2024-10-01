# Payable

Decentralized payment as a service.

## Use cases

- Subscription payment
- Invoice payment
- Payroll management

## Requirement

- Payee can provide a payment payable to payer
- Payer can deposit on-chain assets for payment of payable after a stipulated period
- Payer can cancel payment of a payable before a cancel period is over
- Payer must deposit enough assets to cover the payment of a payable
- Payer can pay recurrently on a payable from a payee
- Payer can cancel recurrent payment
- Payee can receive payment in `any` on-chain asset with sufficient liquidity
- Payer can pay in `any` on-chain asset with sufficient liquidity
- All transactions related to payment can be verified

## Architecture

### Flow

It describes how the system handles payment settlement between a payee and a payer

payee → `submit payable`

system → `record payable param` + `emit payable event`

payer → `accepts payable` + `deposit enough asset` 

system → `emit acceptance event`

system → `open cancel period`

{ ***if:** `open cancel period` is not over, the payer can cancel the payment*

***else:** if the payer `cancel payment`, 50% of the payable will be paid to the payee }*

system → ***If:*** `payment period` is reached, `transfer` the correct asset to the payee + `emit event`

### Data structure

#### Onchain Payable

```rust
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
```

#### Counter

```rust
pub struct Counter {
    pub payable_idx_counter: u64,
}
```

### Accounts

```rust
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
```

### Events

```rust
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
```

### Errors

```rust
#[error_code]
pub enum Error {
    CyclicPayable,
    WithdrawalTimeNotReached,
    CompletedPayable,
}
```

### Interface

```rust
trait Payable {
    pub fn initialize(ctx: Context<Initialize>) -> Result<()>;
    pub fn create_payable(
        ctx: Context<CreatePayable>,
        amount: i64,
        recurrent: bool,
        number_of_recurrent_payment: i64,
        recurrent_payment_interval: i64,
        cancel_period: i64,
    ) -> Result<()>;
    pub fn accept_payable(ctx: Context<AcceptPayable>, recurrent: bool) -> Result<()>;
    pub fn cancel_payable(ctx: Context<CancelPayable>) -> Result<()>;
    pub fn withdraw(ctx: Context<WithdrawFromPayable>) -> Result<()>;
}
```

## Develop

### Install Rust and Anchor

- Install rust

 ```bash
 curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
 ```

- Install anchor

Follow the instuction [here](https://book.anchor-lang.com/getting_started/installation.html)

### Build

- Open a teminal and run

```bash
anchor build
```

### Test

- Open a separate terminal and run solana local validator

```bash
solana-test-validator -r 
```

- Use localnet in solana config

```bash
solana config set -u localhost
```

- Open a separate terminal and run test

```bash
anchor test --skip-local-validator
```

### Deploy to localnet

- Run the command below

```bash
anchor deploy
```

### Deploy to devnet

- Edit **Anchor.toml**

1. cluster = "devnet"
2. wallet = "~/.config/solana/id.json" is the location of your solana key pair
3.[programs.devnet]

Your Anchor.toml should look like so:

```bash
[toolchain]

[features]
seeds = false
skip-lint = false

[programs.devnet]
payable = "8rrFrtdFK3x8NBFEvaqznHg9Q9Tf2ij6bHEv2FCHotgJ"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "devnet"
wallet = "/Users/cenwadike/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

- Config solana to devnet by running this command

```bash
solana config set --keypair ~/.config/solana/id.json  --url devnet
```

- View solana config and confirm it's on devnet

```bash 
solana config get 
```

- Run the command to get testnet SOL

```bash
solana airdrop 5
```

- Run the command to deploy to testnet

```bash
anchor deploy
```

- View deployed contract using this command

```bash
solana program show <Program Id>
```

## Devnet contract address

```bash
8rrFrtdFK3x8NBFEvaqznHg9Q9Tf2ij6bHEv2FCHotgJ
```
