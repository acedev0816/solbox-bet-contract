use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("BWVuwvBhFG3nnUexc5CGCzGwELP3eK9oymPCtUiDQXxC");

#[program]
pub mod solbet_contract {
    use super::*;

    pub fn init(
        _ctx: Context<Init>,
        _escrow_account_bump:u8
    ) -> Result<()> 
    {
        msg!("Acer 6/27");
        _ctx.accounts.vault_account.admin_key = *_ctx.accounts.payer.key;
        Ok(())
    }

    pub fn set_admin(
        ctx: Context<SetAdmin>,
    ) -> Result<()>
    {
        ctx.accounts.vault_account.admin_key = *ctx.accounts.new_admin.key;
        Ok(())
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        deposit_amount: u64
    ) -> Result<()>
    {
        //transfer SOL to vault
        let cpi_ctx = CpiContext::new (
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.depositor.to_account_info(),
                to: ctx.accounts.vault_account.to_account_info()
            }
        );
        transfer(cpi_ctx, deposit_amount)?;
        
        Ok(())
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        withdraw_amount: u64
    ) -> Result<()>
    {
        //transfer SOL from vault to owner
        let from:AccountInfo = ctx.accounts.vault_account.to_account_info();
        let to:AccountInfo = ctx.accounts.withdrawer.to_account_info();
        **from.try_borrow_mut_lamports()? -= withdraw_amount;
        **to.try_borrow_mut_lamports()? += withdraw_amount;

        Ok(())
    }

    pub fn create_bet_account(
        ctx: Context<CreateBetAccount>,
        bet_account_bump:u8
    ) -> Result<()> 
    {
        ctx.accounts.bet_account.prize_amount = 0;
        Ok(())
    }

    pub fn claim_prize(
        ctx: Context<ClaimPrize>
    ) -> Result<()>
    {
        let from:AccountInfo = ctx.accounts.bet_account.to_account_info();
        let to:AccountInfo = ctx.accounts.player.to_account_info();
        **from.try_borrow_mut_lamports()? -= ctx.accounts.bet_account.prize_amount;
        **to.try_borrow_mut_lamports()? += ctx.accounts.bet_account.prize_amount;

        ctx.accounts.bet_account.prize_amount = 0;
        Ok(())
    }

    pub fn bet(
        ctx: Context<Bet>,
        amount: u64,
    ) -> Result<()> {
        
        // transfer SOL to vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.player.to_account_info(),
                to: ctx.accounts.vault_account.to_account_info()
            }
        );
        transfer(cpi_ctx, amount)?;

        // calc lottery result
        let now = Clock::get().unwrap().unix_timestamp as u64;
        // let mut rng = rand::thread_rng();
        let mut rand = (now%4444 + (now%2)*5000 + (now%5)*2000 + (now%7) * 1000)%10000;
        rand = rand * 10;

        let mut prize_amount;
        msg!("rand value: {}", rand);
        // 0x              39.155%
        // 0.5x           24.675%
        // 1x               20.480%
        // 2x                8.150%
        // 5x                5.150%
        // 10x              2.075%
        // 25x             0.250%
        // 50x             0.035%
        // 100x           0.020%
        // 125x            0.010% 

        if rand < 39155 {
            prize_amount = 0;
        } else if rand < 63830{
            prize_amount = amount/2;
        } else if rand < 84310 {
            prize_amount = amount;
        } else if rand < 92460
        {
            prize_amount = amount * 2;
        } else if rand < 97610{
            prize_amount = amount * 5;
        } else if rand <99685{
            prize_amount = amount * 10;
        } else if rand < 99935{
            prize_amount = amount * 25;
        } else if rand < 99970{
            prize_amount = amount * 50;
        } else if rand < 99990{
            prize_amount = amount * 100;
        } else {
            prize_amount = amount * 125;
        }
        //test
        // prize_amount = amount *2;
        msg!("prize_amount: {}", prize_amount);
        // send prize
        let from:AccountInfo = ctx.accounts.vault_account.to_account_info();
        let to:AccountInfo = ctx.accounts.bet_account.to_account_info();
        **from.try_borrow_mut_lamports()? -= prize_amount;
        **to.try_borrow_mut_lamports()? += prize_amount;

        ctx.accounts.bet_account.prize_amount += prize_amount;

        ctx.accounts.bet_account.player = *ctx.accounts.player.key;
        // emit event
        emit!(BetResult{
            player: *ctx.accounts.player.to_account_info().key,
            amount:  amount,
            prize_amount: prize_amount,
            ts: now
        });

        Ok(())
    }
}


/////////////////////////////////////////////
/// Contexts
/////////////////////////////////////////////

#[derive(Accounts)]
#[instruction(valut_account_bump: u8)]
pub struct Init<'info>
{
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        init,
        seeds = [b"vault".as_ref()],
        bump,
        payer = payer,
        space = 8 + VaultAccount::LEN
    )]
    pub vault_account: Account<'info, VaultAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>
}


#[derive(Accounts)]
#[instruction(bet_account_bump: u8)]
pub struct CreateBetAccount<'info>
{
    #[account(
        init,
        seeds = [player.key().as_ref()],
        bump,
        space = 8 + BetAccount::LEN,
        payer = player
     )]
     pub bet_account: Account<'info, BetAccount>,
     
     #[account(mut)]
     pub player: Signer<'info>,
     pub system_program: Program<'info, System>

}


#[derive(Accounts)]
#[instruction(bet_amount:u64)]
pub struct Bet<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut, 
        signer,
        constraint = player.lamports() > bet_amount
    )]
    pub player: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault_account: AccountInfo<'info>,
    #[account(mut)]
    pub bet_account: Account<'info, BetAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut, 
        signer,
    )]
    pub player: AccountInfo<'info>,
    #[account(
        mut,
        has_one = player
    )]
    pub bet_account: Account<'info, BetAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetAdmin<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut, 
        signer,
        constraint = vault_account.admin_key == *admin.key
    )]
    pub admin: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub new_admin: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
    )]
    pub vault_account: Account<'info, VaultAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(deposit_amount:u64)]
pub struct Deposit<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut, 
        signer,
        constraint = depositor.lamports() > deposit_amount
    )]
    pub depositor: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault_account: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(withdraw_amount:u64)]
pub struct Withdraw<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut, 
        signer,
        constraint = vault_account.admin_key == *withdrawer.key
    )]
    pub withdrawer: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = vault_account.to_account_info().lamports() > withdraw_amount
    )]
    pub vault_account: Account<'info, VaultAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
}



//////////////////////////////////////////
/// Account Structures
//////////////////////////////////////////

// Vault Account
#[account]
#[derive(Default)]
pub struct VaultAccount {
    pub admin_key: Pubkey
}
impl VaultAccount {
    pub const LEN: usize = 32;
}

//Bet Account
#[account]
#[derive(Default)]
pub struct BetAccount {
    pub prize_amount: u64,
    pub player: Pubkey
}

impl BetAccount{
    pub const LEN: usize = 64 + 64 + 32;
}

//////////////////////////////////////////
/// Events
//////////////////////////////////////////
#[event]
pub struct BetResult {
    pub player: Pubkey,
    pub amount: u64,
    pub prize_amount: u64,
    pub ts: u64
}
