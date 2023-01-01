use crate::error::ContractError;
use crate::state::GenericBalance;

use cosmwasm_std::coins;
use cosmwasm_std::{to_binary, Addr, BankMsg, CosmosMsg, DepsMut, Empty, StdResult, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw721::Cw721ExecuteMsg;

// Actual community pool on mainnet
const COMMUNITY_POOL: &str = "juno1jv65s3grqf6v6jl3dp4t6c9t9rk99cd83d88wr";
// use fake contract address for testnet
//const COMMUNITY_POOL: &str = ""

pub fn send_tokens_cosmos(to: &Addr, balance: &GenericBalance) -> StdResult<Vec<CosmosMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<CosmosMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![CosmosMsg::from(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.clone(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            // Only works if recipient is User Address, doesn't work for DAO / Contracts
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = CosmosMsg::from(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();

    msgs.extend(cw20_msgs?);

    let nft_balance = &balance.nfts;
    let nft_msgs: StdResult<Vec<CosmosMsg<Empty>>> = nft_balance
        .iter()
        .map(|n| {
            let msg = Cw721ExecuteMsg::TransferNft {
                recipient: to.into(),
                token_id: n.token_id.clone(),
            };
            let exec = CosmosMsg::from(WasmMsg::Execute {
                contract_addr: n.contract_address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();

    msgs.extend(nft_msgs?);

    Ok(msgs)
}

// Validate Ask
// Removes any 0 values, returns error if duplicate is found
pub fn normalize_ask_error_on_dup(ask: GenericBalance) -> Result<GenericBalance, ContractError> {
    let mut normalized = ask;

    // Remove 0 values
    normalized.native.retain(|c| c.amount.u128() != 0);

    let dup_check = |mut val: GenericBalance| -> Result<(), ContractError> {
        // Sort
        val.native.sort_unstable_by(|a, b| a.denom.cmp(&b.denom));
        // Length of original
        let len_pre_dedup = val.native.len();
        // Dedup
        val.native.dedup_by_key(|i| i.denom.clone());
        // Length after dedup
        let len_post_dedup = val.native.len();
        // If lengths different, dupes were found so return error
        if len_pre_dedup != len_post_dedup {
            return Err(ContractError::GenericError("Duplicates found in Ask Price".to_string()));
        };

        Ok(())
    };

    dup_check(normalized.clone())?;
    Ok(normalized)
}

pub fn calc_fee(balance: &GenericBalance) -> StdResult<Option<(CosmosMsg, GenericBalance)>> {
    let juno_in_balance = balance.native.iter().find(|n| n.denom == *"ujunox");

    // If balance DOES NOT contain juno, return Ok(None)
    // If balance DOES contain juno, calculate 0.1% of the JUNO in the balance,
    // Create CosmosMsg sending that to the Community Pool,
    // and return this CosmosMsg + a generic balance with the fee removed for the user
    if let Some(juno) = juno_in_balance {
        // 0.1% = amount * 1 / 1000
        let ten_pips = juno.amount.multiply_ratio(1_u128, 1000_u128);

        // small amounts (like 1ujuno) will be 0
        if ten_pips.is_zero() {
            return Ok(None);
        }

        let fee_msg: CosmosMsg<Empty> = CosmosMsg::from(BankMsg::Send {
            to_address: COMMUNITY_POOL.to_string(),
            amount: coins(ten_pips.u128(), "ujunox"),
        });

        let juno_amount_after_fee_removed = juno.amount.checked_sub(ten_pips)?;

        let balance_with_fee_removed = {
            let mut x = balance.clone();
            x.native.retain(|n| n.denom != *"ujunox");
            x.native.append(&mut coins(juno_amount_after_fee_removed.u128(), "ujunox"));
            x
        };

        Ok(Some((fee_msg, balance_with_fee_removed)))
    } else {
        Ok(None)
    }
}

/// Get allowed purchasers for a given listing.
/// If any address string is not valid, returns an error
pub fn get_whitelisted_addresses(
    deps: &DepsMut,
    whitelisted_addrs: Option<Vec<String>>,
) -> Result<Option<Vec<Addr>>, ContractError> {
    let Some(addrs) = whitelisted_addrs else {
        return Ok(None);
    };

    if addrs.is_empty() {
        return Ok(None);
    };

    let valid: Vec<Addr> = addrs
        .iter()
        .map(|address| {
            deps.api.addr_validate(address).map_err(|_foo| ContractError::InvalidAddressFormat)
        })
        .collect::<Result<Vec<Addr>, ContractError>>()?;

    Ok(Some(valid))
}
