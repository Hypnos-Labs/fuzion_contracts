pub mod contract;
pub mod error;
pub use crate::error::ContractError;
pub mod execute;
pub mod integration_tests;
pub mod msg;
pub mod query;
pub mod state;
pub mod utils;

mod contract_imports {
    pub use cosmwasm_std::{
        entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
        StdResult,
    };
    pub use cw2::set_contract_version;
    pub use cw20::{Balance, Cw20CoinVerified, Cw20ReceiveMsg};
    pub use cw721::Cw721ReceiveMsg;

    pub use crate::error::ContractError;
    pub use crate::execute::{
        execute_add_to_bucket, execute_add_to_bucket_cw721, execute_add_to_listing,
        execute_add_to_listing_cw721, execute_buy_listing, execute_change_ask,
        execute_create_bucket, execute_create_bucket_cw721, execute_create_listing,
        execute_create_listing_cw721, execute_delete_listing, execute_finalize,
        execute_withdraw_bucket, execute_withdraw_purchased,
    };
    pub use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, ReceiveNftMsg};
    pub use crate::query::*;
    //get_all_listings, get_buckets, get_listings_by_owner, get_listings_for_market, get_counts
    pub use crate::state::{FeeDenom, Nft, BUCKET_COUNT, FEE_DENOM, LISTING_COUNT};
}

mod execute_imports {
    pub use crate::error::ContractError;
    pub use crate::msg::CreateListingMsg;
    pub use crate::state::{
        genbal_cmp, listingz, BalanceUtil, Bucket, FeeDenom, GenericBalance, Listing, Nft, Status,
        BUCKETS, BUCKET_COUNT, FEE_DENOM, LISTING_COUNT,
    };
    pub use crate::utils::{
        calc_fee_coin,
        send_tokens_cosmos, 
        proto_encode
            //calc_fee, check_whitelist, check_valid_genbal
         //check_buyer_whitelisted, get_whitelisted_addresses, get_whitelisted_buyers, normalize_ask,
    };
    pub use cosmwasm_std::{Addr, DepsMut, Env, Response, StdError};
    pub use cw20::Balance;
    pub use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as SdkCoin;
    pub use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgFundCommunityPool;
}

mod integration_tests_imports {
    pub use anyhow::ensure;
    pub use core::fmt::Display;

    pub use crate::{msg::*, state::*};
    pub use cosmwasm_std::{coins, to_binary, Addr, Coin, Empty, Uint128}; //BlockInfo;
    pub use cw20::{Cw20Coin, Cw20CoinVerified, Cw20Contract};
}

mod msg_imports {
    pub use crate::query::*;
    pub use cosmwasm_schema::{cw_serde, QueryResponses};
    pub use cw20::Cw20ReceiveMsg;
    pub use cw721::Cw721ReceiveMsg;
    //GetBucketsResponse, MultiListingResponse, CountResponse
    pub use crate::state::GenericBalance;
}

mod query_imports {
    pub use crate::state::{
        listingz, Bucket, FeeDenom, Listing, Status, BUCKETS, BUCKET_COUNT, FEE_DENOM,
        LISTING_COUNT,
    };
    pub use cosmwasm_schema::cw_serde;
    pub use cosmwasm_std::{Addr, Deps, Env, Order, StdError, StdResult};
    pub use cw_storage_plus::PrefixBound;
}

mod state_imports {
    pub use crate::error::ContractError;
    pub use crate::utils::send_tokens_cosmos;
    pub use cosmwasm_schema::cw_serde;
    pub use cosmwasm_std::{Addr, Coin, CosmosMsg, Timestamp, Uint128};
    pub use cw20::{Balance, Cw20CoinVerified};
    pub use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, UniqueIndex};
    pub use std::collections::BTreeMap;
}

mod utils_imports {
    pub use crate::error::ContractError;
    pub use crate::state::{FeeDenom, GenericBalance, Listing};
    pub use cosmwasm_std::{
        coin, coins, to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Empty, StdError,
        StdResult, WasmMsg,
    };
    pub use cw20::Cw20ExecuteMsg;
    pub use cw721::Cw721ExecuteMsg;
    pub use std::collections::BTreeMap;
    pub use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgFundCommunityPool;
}
