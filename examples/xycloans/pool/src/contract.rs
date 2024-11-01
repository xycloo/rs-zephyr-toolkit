use crate::{
    balance::{burn_shares, mint_shares},
    checks::{check_amount_gt_0, check_balance_ge_supply},
    compute_fee, events,
    execution::{invoke_receiver, invoke_receiver_moderc3156},
    rewards::{pay_matured, update_rewards},
    storage::*,
    token_utility::{get_token_client, transfer, transfer_in_pool, try_repay},
    types::Error,
};
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[cfg(feature = "mercury")]
mod retroshade {
    use retroshade_sdk::Retroshade;
    use soroban_sdk::{contracttype, Address, Symbol};

    #[derive(Retroshade)]
    #[contracttype]
    pub struct NewLiquidityEvent {
        pub from: Address,
        pub kind: Symbol,
        pub amount: i128,
        pub at_fee_per_share_universal: i128,
        pub at_fee_per_share_particular: i128,
        pub at_shares: i128,
        pub new_shares_minted: i128,
        pub ledger: u32,
        pub timestamp: u64,
    }
}

#[contract]
pub struct Pool;

pub trait FlashLoanModErc3156 {
    /// The entry point for executing a flash loan, the initiator (or borrower) provides:
    /// `receiver_id: Address` The address of the receiver contract which contains the borrowing logic.
    /// `amount` Amount of `token_id` to borrow (`token_id` is set when the contract is initialized).
    fn borrow_erc(
        e: Env,
        initiator: Address,
        receiver_id: Address,
        amount: i128,
    ) -> Result<(), Error>;
}

pub trait FlashLoan {
    /// The entry point for executing a flash loan, the initiator (or borrower) provides:
    /// `receiver_id: Address` The address of the receiver contract which contains the borrowing logic.
    /// `amount` Amount of `token_id` to borrow (`token_id` is set when the contract is initialized).
    fn borrow(e: Env, receiver_id: Address, amount: i128) -> Result<(), Error>;
}

pub trait Vault {
    /// deposit

    /// Allows to deposit into the pool and mints liquidity provider shares to the lender.
    /// This action currently must be authorized by the `admin`, so the proxy contract.
    /// This allows a pool to be only funded when the pool is part of the wider protocol, and is not an old pool.
    /// This design decision may be removed in the next release, follow https://github.com/xycloo/xycloans/issues/16

    /// `deposit()` must be provided with:
    /// `from: Address` Address of the liquidity provider.
    /// `amount: i128` Amount of `token_id` that `from` wants to deposit in the pool.
    fn deposit(env: Env, from: Address, amount: i128) -> Result<(), Error>;

    /// update_fee_rewards

    /// Updates the matured rewards for a certain user `addr`
    /// This function may be called by anyone.

    /// `update_fee_rewards()` must be provided with:
    /// `addr: Address` The address that is udpating its fee rewards.
    fn update_fee_rewards(e: Env, addr: Address) -> Result<(), Error>;

    /// withdraw_matured

    /// Allows a certain user `addr` to withdraw the matured fees.
    /// Before calling `withdraw_matured()` the user should call `update_fee_rewards`.
    /// If not, the matured fees that were not updated will not be lost, just not included in the payment.

    /// `withdraw_matured()` must be provided with:
    /// `addr: Address` The address that is withdrawing its fee rewards.
    fn withdraw_matured(e: Env, addr: Address) -> Result<(), Error>;

    /// withdraw

    /// Allows to withdraw liquidity from the pool by burning liquidity provider shares.
    /// Will result in a cross contract call to the flash loan, which holds the funds that are being withdrawn.
    /// The liquidity provider can also withdraw only a portion of its shares.

    /// withdraw() must be provided with:
    /// `addr: Address` Address of the liquidity provider
    /// `amount: i28` Amount of shares that are being withdrawn
    fn withdraw(env: Env, addr: Address, amount: i128) -> Result<(), Error>;

    /// Returns the amount of shares that an address holds.
    fn shares(e: Env, addr: Address) -> i128;

    /// Returns the amount of matured fees for an address.
    fn matured(env: Env, addr: Address) -> i128;
}

pub trait Initializable {
    /// initialize

    /// Constructor function, only to be callable once

    /// `initialize()` must be provided with:
    /// `token_id: Address` The pool's token.
    /// `flash_loan` The address of the associated flash loan contract. `flash_loan` should have `current_contract_address()` as `lp`.
    fn initialize(env: Env, token: Address) -> Result<(), Error>;
}

#[contractimpl]
impl Initializable for Pool {
    fn initialize(env: Env, token: Address) -> Result<(), Error> {
        if has_token_id(&env) {
            return Err(Error::AlreadyInitialized);
        }

        put_token_id(&env, token);
        Ok(())
    }
}

#[contractimpl]
impl Vault for Pool {
    fn deposit(env: Env, from: Address, amount: i128) -> Result<(), Error> {
        check_amount_gt_0(amount)?;

        from.require_auth();

        bump_instance(&env);

        #[cfg(feature = "mercury")]
        let (at_fee_per_share_particular, at_fee_per_share_universal, at_shares) = (
            read_fee_per_share_particular(&env, from.clone()),
            get_fee_per_share_universal(&env),
            get_tot_supply(&env),
        );

        // we update the rewards before the deposit to avoid the abuse of the collected fees by withdrawing them with liquidity that didn't contribute to their generation.
        update_rewards(&env, from.clone());

        // transfer the funds into the flash loan
        let token_client = get_token_client(&env);
        transfer_in_pool(&env, &token_client, &from, &amount);
        //transfer_into_flash_loan(&e, &token_client, &from, &amount);

        // mint the new shares to the lender.
        // shares to mint will always be the amount deposited, see https://github.com/xycloo/xycloans/issues/17
        mint_shares(&env, from.clone(), amount);

        events::deposited(&env, from.clone(), amount);

        #[cfg(feature = "mercury")]
        retroshade::NewLiquidityEvent {
            from,
            amount,
            at_fee_per_share_particular,
            at_fee_per_share_universal,
            at_shares,
            new_shares_minted: amount,
            kind: symbol_short!("deposit"),
            ledger: env.ledger().sequence(),
            timestamp: env.ledger().timestamp(),
        }
        .emit(&env);

        Ok(())
    }

    fn withdraw_matured(env: Env, addr: Address) -> Result<(), Error> {
        // require lender auth for withdrawal
        addr.require_auth();

        bump_instance(&env);

        // pay the matured yield
        let paid = pay_matured(&env, addr.clone())?;

        // ensure that the pool's balance is >= total supply
        check_balance_ge_supply(&env, &get_token_client(&env))?;

        events::matured_withdrawn(&env, addr, paid);
        Ok(())
    }

    fn update_fee_rewards(env: Env, addr: Address) -> Result<(), Error> {
        bump_instance(&env);

        update_rewards(&env, addr);

        Ok(())
    }

    fn withdraw(env: Env, addr: Address, amount: i128) -> Result<(), Error> {
        check_amount_gt_0(amount)?;

        // require lender auth for withdrawal
        addr.require_auth();

        bump_instance(&env);

        #[cfg(feature = "mercury")]
        let (at_fee_per_share_particular, at_fee_per_share_universal, at_shares) = (
            read_fee_per_share_particular(&env, addr.clone()),
            get_fee_per_share_universal(&env),
            get_tot_supply(&env),
        );

        let addr_balance = read_balance(&env, addr.clone());

        // if the desired burned shares are more than the lender's balance return an error
        // if the amount is 0 return an error
        if addr_balance < amount || amount == 0 {
            return Err(Error::InvalidShareBalance);
        }

        // update addr's rewards
        update_rewards(&env, addr.clone());

        // pay out the corresponding deposit
        let token_client = get_token_client(&env);
        transfer(&env, &token_client, &addr, &amount);

        // burn the shares
        burn_shares(&env, addr.clone(), amount);

        #[cfg(feature = "mercury")]
        retroshade::NewLiquidityEvent {
            from: addr.clone(),
            amount,
            at_fee_per_share_particular,
            at_fee_per_share_universal,
            at_shares,
            new_shares_minted: amount,
            kind: symbol_short!("withdraw"),
            ledger: env.ledger().sequence(),
            timestamp: env.ledger().timestamp(),
        }
        .emit(&env);

        events::withdrawn(&env, addr, amount);
        Ok(())
    }

    fn shares(e: Env, addr: Address) -> i128 {
        read_balance(&e, addr)
    }

    fn matured(env: Env, addr: Address) -> i128 {
        read_matured_fees_particular(&env, addr)
    }
}

#[cfg(feature = "moderc3156")]
#[contractimpl]
impl FlashLoanModErc3156 for Pool {
    fn borrow_erc(
        env: Env,
        initiator: Address,
        receiver_id: Address,
        amount: i128,
    ) -> Result<(), Error> {
        initiator.require_auth();
        check_amount_gt_0(amount)?;

        bump_instance(&env);

        let client = get_token_client(&env);

        // transfer `amount` to `receiver_id`
        transfer(&env, &client, &receiver_id, &amount);

        // invoke the `exec_op()` function of the receiver contract
        let fee = compute_fee(&amount);

        invoke_receiver_moderc3156(&env, &receiver_id, &client.address, &amount, &fee);

        // try `transfer_from()` of (`amount` + fees) from the receiver to the flash loan
        try_repay(&env, &client, &receiver_id, amount, fee)?;

        events::loan_successful(&env, receiver_id, amount);
        Ok(())
    }
}

//#[cfg(not(feature="moderc3156"))]
#[contractimpl]
impl FlashLoan for Pool {
    fn borrow(env: Env, receiver_id: Address, amount: i128) -> Result<(), Error> {
        check_amount_gt_0(amount)?;

        bump_instance(&env);

        let client = get_token_client(&env);

        // transfer `amount` to `receiver_id`
        transfer(&env, &client, &receiver_id, &amount);

        // invoke the `exec_op()` function of the receiver contract
        let fee = compute_fee(&amount);

        invoke_receiver(&env, &receiver_id);

        // try `transfer_from()` of (`amount` + fees) from the receiver to the flash loan
        try_repay(&env, &client, &receiver_id, amount, fee)?;

        events::loan_successful(&env, receiver_id, amount);
        Ok(())
    }
}
