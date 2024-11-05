use inner::Pool;
use serde::{Deserialize, Serialize};
use soroban_sdk::{map, vec};
use storage::{PoolConfig, PoolDataKey, PositionData, Positions};

use crate::{utils::address_from_str, EnvClient};

pub const SCALAR_9: i128 = 1_000_000_000;
pub const SCALAR_7: i128 = 1_0000000;


#[derive(Serialize, Deserialize, Debug)]
pub struct BlendHfResponse {
    pub min: i64,
    pub current: i64,
}

pub mod storage {
    use soroban_sdk::{contracttype, Map, Vec};
    use soroban_sdk::{
        xdr::{ContractDataEntry, LedgerEntryData, ScContractInstance, ScVal}, Address, Symbol, TryFromVal
    };
    use crate::EnvClient;
    use super::*;

    #[derive(Clone)]
    #[contracttype]
    pub struct Reserve {
        pub asset: Address,        // the underlying asset address
        pub index: u32,            // the reserve index in the pool
        pub l_factor: u32,         // the liability factor for the reserve
        pub c_factor: u32,         // the collateral factor for the reserve
        pub max_util: u32,         // the maximum utilization rate for the reserve
        pub last_time: u64,        // the last block the data was updated
        pub scalar: i128,          // scalar used for positions, b/d token supply, and credit
        pub d_rate: i128,          // the conversion rate from dToken to underlying (9 decimals)
        pub b_rate: i128,          // the conversion rate from bToken to underlying (9 decimals)
        pub ir_mod: i128,          // the interest rate curve modifier (9 decimals)
        pub b_supply: i128,        // the total supply of b tokens
        pub d_supply: i128,        // the total supply of d tokens
        pub backstop_credit: i128, // the total amount of underlying tokens owed to the backstop
    }

    #[derive(Clone)]
    #[contracttype]
    pub struct Request {
        pub request_type: u32,
        pub address: Address, // asset address or liquidatee
        pub amount: i128,
    }

    #[derive(Clone, PartialEq)]
    #[repr(u32)]
    pub enum RequestType {
        Supply = 0,
        Withdraw = 1,
        SupplyCollateral = 2,
        WithdrawCollateral = 3,
        Borrow = 4,
        Repay = 5,
        FillUserLiquidationAuction = 6,
        FillBadDebtAuction = 7,
        FillInterestAuction = 8,
        DeleteLiquidationAuction = 9,
    }

    impl RequestType {
        /// Convert a u32 to a RequestType
        ///
        /// ### Panics
        /// If the value is not a valid RequestType
        pub fn from_u32(value: u32) -> Self {
            match value {
                0 => RequestType::Supply,
                1 => RequestType::Withdraw,
                2 => RequestType::SupplyCollateral,
                3 => RequestType::WithdrawCollateral,
                4 => RequestType::Borrow,
                5 => RequestType::Repay,
                6 => RequestType::FillUserLiquidationAuction,
                7 => RequestType::FillBadDebtAuction,
                8 => RequestType::FillInterestAuction,
                9 => RequestType::DeleteLiquidationAuction,
                _ => RequestType::Supply,
            }
        }
    }

    #[derive(Clone)]
    #[contracttype]
    pub enum PoolDataKey {
        // A map of underlying asset's contract address to reserve config
        ResConfig(Address),
        // A map of underlying asset's contract address to queued reserve init
        ResInit(Address),
        // A map of underlying asset's contract address to reserve data
        ResData(Address),
        // The reserve's emission config
        EmisConfig(u32),
        // The reserve's emission data
        EmisData(u32),
        // Map of positions in the pool for a user
        Positions(Address),
    }

    #[derive(Clone, Debug)]
    #[contracttype]
    pub struct Positions {
        pub liabilities: Map<u32, i128>, // Map of Reserve Index to liability share balance
        pub collateral: Map<u32, i128>,  // Map of Reserve Index to collateral supply share balance
        pub supply: Map<u32, i128>, // Map of Reserve Index to non-collateral supply share balance
    }

    #[derive(Clone, Debug)]
    #[contracttype]
    pub struct ReserveData {
        pub d_rate: i128, // the conversion rate from dToken to underlying expressed in 9 decimals
        pub b_rate: i128, // the conversion rate from bToken to underlying expressed with the underlying's decimals
        pub ir_mod: i128, // the interest rate curve modifier
        pub b_supply: i128, // the total supply of b tokens
        pub d_supply: i128, // the total supply of d tokens
        pub backstop_credit: i128, // the amount of underlying tokens currently owed to the backstop
        pub last_time: u64, // the last block the data was updated
    }

    #[derive(Clone, Debug)]
    #[contracttype]
    pub struct ReserveConfig {
        pub index: u32,      // the index of the reserve in the list
        pub decimals: u32,   // the decimals used in both the bToken and underlying contract
        pub c_factor: u32,   // the collateral factor for the reserve scaled expressed in 7 decimals
        pub l_factor: u32,   // the liability factor for the reserve scaled expressed in 7 decimals
        pub util: u32,       // the target utilization rate scaled expressed in 7 decimals
        pub max_util: u32,   // the maximum allowed utilization rate scaled expressed in 7 decimals
        pub r_base: u32, // the R0 value (base rate) in the interest rate formula scaled expressed in 7 decimals
        pub r_one: u32,  // the R1 value in the interest rate formula scaled expressed in 7 decimals
        pub r_two: u32,  // the R2 value in the interest rate formula scaled expressed in 7 decimals
        pub r_three: u32, // the R3 value in the interest rate formula scaled expressed in 7 decimals
        pub reactivity: u32, // the reactivity constant for the reserve scaled expressed in 7 decimals
    }

    pub struct PositionData {
        /// The effective collateral balance denominated in the base asset
        pub collateral_base: f64,
        // The raw collateral balance demoninated in the base asset
        pub collateral_raw: i128,
        /// The effective liability balance denominated in the base asset
        pub liability_base: f64,
        // The raw liability balance demoninated in the base asset
        pub liability_raw: i128,
        /// The scalar for the base asset
        pub scalar: i128,
    }

    #[derive(Clone)]
    #[contracttype]
    pub struct PoolConfig {
        pub oracle: Address,    // the contract address of the oracle
        pub bstop_rate: u32, // the rate the backstop takes on accrued debt interest, expressed in 7 decimals
        pub status: u32,     // the status of the pool
        pub max_positions: u32, // the maximum number of effective positions (collateral + liabilities) a single user can hold
    }

    const POOL_CONFIG_KEY: &str = "Config";
    pub fn get_res_config(pool: [u8; 32], asset: &Address) -> ReserveConfig {
        let key = PoolDataKey::ResConfig(asset.clone());
        EnvClient::empty()
            .read_contract_entry_by_key::<PoolDataKey, ReserveConfig>(pool, key)
            .unwrap()
            .unwrap()
    }

    pub fn get_res_data(pool: [u8; 32], asset: &Address) -> ReserveData {
        let key = PoolDataKey::ResData(asset.clone());

        EnvClient::empty()
            .read_contract_entry_by_key::<PoolDataKey, ReserveData>(pool, key)
            .unwrap()
            .unwrap()
    }

    pub fn get_pool_config(pool: [u8; 32]) -> Option<PoolConfig> {
        let env = EnvClient::empty();
        let instance = env.read_contract_instance(pool).unwrap().unwrap();

        let LedgerEntryData::ContractData(ContractDataEntry { val, .. }) = instance.entry.data
        else {
            panic!()
        };

        let ScVal::ContractInstance(ScContractInstance { storage, .. }) = val else {
            panic!()
        };

        for kv in storage.unwrap().0.to_vec() {
            if env.from_scval::<Symbol>(&kv.key) == Symbol::new(&env.soroban(), POOL_CONFIG_KEY) {
                return Some(env.from_scval(&kv.val));
            }
        }

        None
    }

    pub fn get_res_list(pool: [u8; 32]) -> Vec<Address> {
        let key = Symbol::new(EnvClient::empty().soroban(), RES_LIST_KEY);
        EnvClient::empty()
            .read_contract_entry_by_key::<Symbol, Vec<Address>>(pool, key)
            .unwrap()
            .unwrap()
    }

    const RES_LIST_KEY: &str = "ResList";
}

pub mod inner {
    use soroban_fixed_point_math::FixedPoint;
    use soroban_sdk::{contracttype, map, vec, Address, Env, Map, Vec};
    use crate::EnvClient;

    use super::{storage::{self, PoolConfig, PositionData, Positions, Reserve}, SCALAR_7, SCALAR_9};

    fn i128(n: u32) -> i128 {
        n as i128
    }

    pub struct Pool {
        pub config: PoolConfig,
        pub reserves: Map<Address, Reserve>,
        pub reserves_to_store: Vec<Address>,
        pub price_decimals: Option<u32>,
        pub prices: Map<Address, i128>,
    }

    impl Reserve {
        pub fn to_asset_from_d_token(&self, d_tokens: i128) -> i128 {
            d_tokens.fixed_mul_ceil(self.d_rate, SCALAR_9).unwrap_or(0)
        }

        pub fn to_asset_from_b_token(&self, b_tokens: i128) -> i128 {
            b_tokens.fixed_mul_floor(self.b_rate, SCALAR_9).unwrap_or(0)
        }

        pub fn to_effective_asset_from_d_token(&self, d_tokens: i128) -> i128 {
            let assets = self.to_asset_from_d_token(d_tokens);
            assets
                .fixed_div_ceil(i128(self.l_factor), SCALAR_7)
                .unwrap_or(0)
        }

        pub fn to_effective_asset_from_b_token(&self, b_tokens: i128) -> i128 {
            let assets = self.to_asset_from_b_token(b_tokens);
            assets
                .fixed_mul_floor(i128(self.c_factor), SCALAR_7)
                .unwrap_or(0)
        }

        /// Fetch the total liabilities for the reserve in underlying tokens
        pub fn total_liabilities(&self) -> i128 {
            self.to_asset_from_d_token(self.d_supply)
        }

        /// Fetch the total supply for the reserve in underlying tokens
        pub fn total_supply(&self) -> i128 {
            self.to_asset_from_b_token(self.b_supply)
        }

        /// Load a Reserve from the ledger and update to the current ledger timestamp.
        ///
        /// **NOTE**: This function is not cached, and should be called from the Pool.
        ///
        /// ### Arguments
        /// * pool_config - The pool configuration
        /// * asset - The address of the underlying asset
        ///
        /// ### Panics
        /// Panics if the asset is not supported, if emissions cannot be updated, or if the reserve
        /// cannot be updated to the current ledger timestamp.
        pub fn load(e: &Env, pool: [u8; 32], pool_config: &PoolConfig, asset: &Address) -> Reserve {
            let reserve_config = storage::get_res_config(pool, asset);
            let reserve_data = storage::get_res_data(pool, asset);
            let reserve = Reserve {
                asset: asset.clone(),
                index: reserve_config.index,
                l_factor: reserve_config.l_factor,
                c_factor: reserve_config.c_factor,
                max_util: reserve_config.max_util,
                last_time: reserve_data.last_time,
                scalar: (10f64.powi(reserve_config.decimals as i32)) as i128,
                d_rate: reserve_data.d_rate,
                b_rate: reserve_data.b_rate,
                ir_mod: reserve_data.ir_mod,
                b_supply: reserve_data.b_supply,
                d_supply: reserve_data.d_supply,
                backstop_credit: reserve_data.backstop_credit,
            };

            EnvClient::empty().log().debug("Reserve", None);
            reserve
        }
    }

    impl Pool {
        /// Load the Pool from the ledger
        pub fn load(e: &soroban_sdk::Env, pool: [u8; 32]) -> Self {
            let pool_config = storage::get_pool_config(pool).unwrap();

            Pool {
                config: pool_config,
                reserves: map![e],
                reserves_to_store: vec![e],
                price_decimals: None,
                prices: map![e],
            }
        }

        /// Load a Reserve from the ledger and update to the current ledger timestamp. Returns
        /// a cached version if it exists.
        ///
        /// ### Arguments
        /// * asset - The address of the underlying asset
        /// * store - If the reserve is expected to be stored to the ledger
        pub fn load_reserve(
            &mut self,
            pool: [u8; 32],
            e: &Env,
            asset: &Address,
            store: bool,
        ) -> Reserve {
            Reserve::load(e, pool, &self.config, asset)
        }

        pub fn load_price_decimals(&mut self) -> u32 {
            7
        }
    }

    impl PositionData {
        /// Calculate the position data for a given set of of positions
        ///
        /// ### Arguments
        /// * pool - The pool
        /// * positions - The positions to calculate the health factor for
        pub fn calculate_from_positions(
            pool_hash: [u8; 32],
            e: &Env,
            pool: &mut Pool,
            positions: &Positions,
        ) -> Self {
            let env = EnvClient::empty();
            let decimals = pool.load_price_decimals();
            let oracle_scalar = 10f64.powi(decimals as i32);
            let reserve_list = storage::get_res_list(pool_hash);

            let mut collateral_base = 0.0;
            let mut liability_base = 0.0;
            let collateral_raw = 0;
            let liability_raw = 0;

            for i in 0..reserve_list.len() {
                let b_token_balance = positions.collateral.get(i).unwrap_or(0);
                let d_token_balance = positions.liabilities.get(i).unwrap_or(0);

                if b_token_balance == 0 && d_token_balance == 0 {
                    continue;
                }

                let reserve = pool.load_reserve(pool_hash, e, &reserve_list.get_unchecked(i), false);

                let asset_base =
                    crate::protocols::reflector::reflector_price(&env, pool.config.oracle.clone(), reserve.asset) as f64;

                let as_asset_b = (b_token_balance as f64 * reserve.b_rate as f64) / SCALAR_9 as f64;
                let as_effective_b = (as_asset_b as f64 * reserve.c_factor as f64) / SCALAR_7 as f64;

                collateral_base += (asset_base * as_effective_b) / oracle_scalar;

                let as_asset_d = (d_token_balance as f64 * reserve.d_rate as f64) / SCALAR_9 as f64;
                let as_effective_d = (as_asset_d as f64 / reserve.l_factor as f64) / SCALAR_7 as f64;

                liability_base += (asset_base * as_effective_d) / oracle_scalar;
            }

            PositionData {
                collateral_base: (collateral_base * SCALAR_7 as f64) as f64,
                collateral_raw,
                liability_base: (liability_base * SCALAR_7 as f64) as f64,
                liability_raw,
                scalar: oracle_scalar as i128,
            }
        }

        /// Return the health factor as a ratio
        pub fn as_health_factor(&self) -> f64 {
            (self.collateral_base / self.liability_base) / SCALAR_7 as f64
        }
    }
}

/// User-friendly wrapper around blend pools.
pub struct BlendPoolWrapper {
    pub str_addr: String,
    pub pool: Pool,
    pub mocked: bool,
}

impl BlendPoolWrapper {
    /// Create a new blend pool or mock one.
    pub fn new(env: &EnvClient, pool: String, mocked: bool) -> Self {
        let pool_hash = stellar_strkey::Contract::from_string(&pool).unwrap().0;
        let pool_obj = if !mocked {
            Pool::load(&env.soroban(), pool_hash)
        } else {
            Pool {
                config: PoolConfig { oracle: address_from_str(env, "CCEVW3EEW4GRUZTZRTAMJAXD6XIF5IG7YQJMEEMKMVVGFPESTRXY2ZAV"), bstop_rate: 2, status: 2, max_positions: 4 },
                reserves: map![env.soroban()],
                reserves_to_store: vec![env.soroban()],
                price_decimals: None,
                prices: map![env.soroban()],
            }
        };

        Self { str_addr: pool, pool: pool_obj, mocked }
    }

    /// Get a user's health factor.
    pub fn get_user_hf(&mut self, env: &EnvClient, user: &str) -> BlendHfResponse {
        if !self.mocked {
            let pool_hash = stellar_strkey::Contract::from_string(&self.str_addr).unwrap().0;
            let user_positions = env.read_contract_entry_by_key::<PoolDataKey, Positions>(
                pool_hash,
                PoolDataKey::Positions(address_from_str(&env, &user)),
            );
            
            let positions_data = PositionData::calculate_from_positions(
                pool_hash,
                &env.soroban(),
                &mut self.pool,
                &user_positions.unwrap().unwrap(),
            );
            let min = (SCALAR_7 as f64 * 1_0000100.0) / SCALAR_7 as f64;
            let current = positions_data.as_health_factor();
        
            BlendHfResponse {
                min: min as i64,
                current: (current) as i64,
            }
        } else {
            BlendHfResponse {
                current: 10070000,
                min: 10000100
            }
        }
    }

    /// Get pool as hash.
    pub fn as_hash(&self) -> [u8; 32] {
        stellar_strkey::Contract::from_string(&self.str_addr).unwrap().0
    }

    /// Get price of an asset in the pool
    pub fn get_price(&self, env: &EnvClient, asset: &str) -> f64 {
        if !self.mocked {
            crate::protocols::reflector::reflector_price(env, self.get_config().oracle, address_from_str(env, asset)) as f64 / SCALAR_7 as f64
        } else {
            1.0
        }
    }

    /// Get the pool's config.
    pub fn get_config(&self) -> PoolConfig {
        self.pool.config.clone()
    }
}

