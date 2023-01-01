use crate::error::ContractError;
use crate::msg::CreateListingMsg;
use crate::state::{
    genbal_from_nft, listingz, Bucket, GenericBalance, GenericBalanceUtil, Listing, Nft, Status,
    ToGenericBalance, BUCKETS,
};
use crate::utils::{
    calc_fee, get_whitelisted_addresses, normalize_ask, send_tokens_cosmos, EzTime, get_whitelisted_buyers, check_buyer_whitelisted,
};
use cosmwasm_std::{Addr, DepsMut, Env, Response};
use cw20::Balance;

//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// Buckets
//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
pub fn execute_create_bucket(
    deps: DepsMut,
    funds: &Balance,
    creator: &Addr,
    bucket_id: &String,
) -> Result<Response, ContractError> {
    // Can't create an empty Bucket
    if funds.is_empty() {
        return Err(ContractError::NoTokens {});
    }

    // Check that bucket_id isn't used
    if BUCKETS.has(deps.storage, (creator.clone(), bucket_id)) {
        return Err(ContractError::IdAlreadyExists {});
    }

    // Save bucket
    BUCKETS.save(
        deps.storage,
        (creator.clone(), bucket_id),
        &Bucket {
            funds: funds.to_generic(),
            owner: creator.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_bucket")
        .add_attribute("bucket_id", bucket_id))
}

pub fn execute_create_bucket_cw721(
    deps: DepsMut,
    user_wallet: &Addr,
    nft: Nft,
    bucket_id: &str,
) -> Result<Response, ContractError> {
    // Check that bucket_id isn't used
    if BUCKETS.has(deps.storage, (user_wallet.clone(), bucket_id)) {
        return Err(ContractError::IdAlreadyExists {});
    }

    // NFT validation checks are handled in receiver wrapper
    // Save bucket
    BUCKETS.save(
        deps.storage,
        (user_wallet.clone(), bucket_id),
        &Bucket {
            funds: genbal_from_nft(nft),
            owner: user_wallet.clone(),
        },
    )?;

    Ok(Response::default())
}

pub fn execute_add_to_bucket(
    deps: DepsMut,
    funds: Balance,
    sender: &Addr,
    bucket_id: String,
) -> Result<Response, ContractError> {
    // Error if no funds sent
    if funds.is_empty() {
        return Err(ContractError::NoTokens {});
    }

    // Ensure bucket exists & Sender is owner
    let Some(the_bucket) = BUCKETS.may_load(deps.storage, (sender.clone(), &bucket_id))? else {
        return Err(ContractError::NotFound { typ: "Bucket".to_string(), id: bucket_id })
    };

    // Authorized check
    if sender != &the_bucket.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Add tokens
    let new_bucket = {
        let old_funds = the_bucket.funds.clone();
        let mut new_bucket = the_bucket;
        new_bucket.funds.add_tokens(funds);
        if old_funds == new_bucket.funds {
            Err(ContractError::ErrorAdding("Tokens to bucket".to_string()))
        } else {
            Ok(new_bucket)
        }
    }?;

    // Save the updated bucket
    BUCKETS.save(deps.storage, (sender.clone(), &bucket_id), &new_bucket)?;

    Ok(Response::new()
        .add_attribute("action", "add_funds_to_bucket")
        .add_attribute("bucket_id", &bucket_id))
}

pub fn execute_add_to_bucket_cw721(
    deps: DepsMut,
    user_wallet: &Addr,
    nft: Nft,
    bucket_id: String,
) -> Result<Response, ContractError> {
    // Ensure bucket exists & Sender is owner
    let Some(the_bucket) = BUCKETS.may_load(deps.storage, (user_wallet.clone(), &bucket_id))? else {
        return Err(ContractError::NotFound { typ: "Bucket".to_string(), id: bucket_id })
    };

    // Authorized check
    if user_wallet != &the_bucket.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Create updated bucket
    let new_bucket = {
        let old_funds = the_bucket.funds.clone();
        let mut new_bucket = the_bucket;
        new_bucket.funds.add_nft(nft);
        if old_funds == new_bucket.funds {
            Err(ContractError::ErrorAdding("NFT to bucket".to_string()))
        } else {
            Ok(new_bucket)
        }
    }?;

    // Save updated bucket
    BUCKETS.update(deps.storage, (user_wallet.clone(), &bucket_id), {
        |o| match o {
            Some(_) => Ok(new_bucket),
            None => Err(ContractError::ToDo {}),
        }
    })?;

    Ok(Response::new()
        .add_attribute("action", "execute_add_to_bucket_cw721")
        .add_attribute("bucket_id", bucket_id))
}

pub fn execute_withdraw_bucket(
    deps: DepsMut,
    user: &Addr,
    bucket_id: &str,
) -> Result<Response, ContractError> {
    // Get Bucket
    let the_bucket = BUCKETS.load(deps.storage, (user.clone(), bucket_id))?;

    // Check sender is owner
    if &the_bucket.owner != user {
        return Err(ContractError::Unauthorized {});
    }

    // Create Send Msgs
    let msgs = send_tokens_cosmos(user, &the_bucket.funds)?;

    // Remove Bucket
    BUCKETS.remove(deps.storage, (user.clone(), bucket_id));

    Ok(Response::new()
        .add_attribute("action", "empty_bucket")
        .add_attribute("bucket_id", bucket_id)
        .add_messages(msgs))
}

//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// Listings
//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub fn execute_create_listing(
    deps: DepsMut,
    user_address: &Addr,
    funds_sent: &Balance,
    createlistingmsg: CreateListingMsg,
) -> Result<Response, ContractError> {
    // Check that some tokens were sent with message
    if funds_sent.is_empty() {
        return Err(ContractError::NoTokens {});
    }

    // Check ID isn't taken
    if (listingz().idx.id.item(deps.storage, createlistingmsg.id.clone())?).is_some() {
        return Err(ContractError::IdAlreadyExists {});
    }

    // let whitelisted_addrs =
    //     get_whitelisted_addresses(&deps, createlistingmsg.whitelisted_purchasers)?;

    let (wl_buyer_one, wl_buyer_two, wl_buyer_three) = 
        get_whitelisted_buyers(
            &deps, 
            createlistingmsg.whitelist_buyer_one,
            createlistingmsg.whitelist_buyer_two,
            createlistingmsg.whitelist_buyer_three
        )?;


    let ask_tokens = normalize_ask(createlistingmsg.ask);

    // Save listing
    listingz().save(
        deps.storage,
        (user_address, createlistingmsg.id.clone()),
        &Listing {
            creator: user_address.clone(),
            id: createlistingmsg.id.clone(),
            finalized_time: None,
            expiration_time: None,
            status: Status::BeingPrepared,
            for_sale: funds_sent.to_generic(),
            ask: ask_tokens,
            claimant: None,
            //whitelisted_purchasers: whitelisted_addrs,
            whitelisted_buyer_one: wl_buyer_one,
            whitelisted_buyer_two: wl_buyer_two,
            whitelisted_buyer_three: wl_buyer_three
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_native_listing")
        .add_attribute("listing_id", &createlistingmsg.id))
}

pub fn execute_create_listing_cw20(
    deps: DepsMut,
    user_address: &Addr,
    _contract_address: &Addr,
    funds_sent: &Balance,
    createlistingmsg: CreateListingMsg,
) -> Result<Response, ContractError> {
    // Check that some tokens were sent with message
    if funds_sent.is_empty() {
        return Err(ContractError::NoTokens {});
    }

    // Checking to make sure that listing_id isn't taken
    if (listingz().idx.id.item(deps.storage, createlistingmsg.id.clone())?).is_some() {
        return Err(ContractError::IdAlreadyExists {});
    }

    // let whitelisted_addrs =
    //     get_whitelisted_addresses(&deps, createlistingmsg.whitelisted_purchasers)?;

    let (wl_buyer_one, wl_buyer_two, wl_buyer_three) = 
        get_whitelisted_buyers(
            &deps, 
            createlistingmsg.whitelist_buyer_one,
            createlistingmsg.whitelist_buyer_two,
            createlistingmsg.whitelist_buyer_three
        )?;

    let ask_tokens = normalize_ask(createlistingmsg.ask);

    listingz().save(
        deps.storage,
        (user_address, createlistingmsg.id.clone()),
        &Listing {
            creator: user_address.clone(),
            id: createlistingmsg.id.clone(),
            finalized_time: None,
            expiration_time: None,
            status: Status::BeingPrepared,
            for_sale: funds_sent.to_generic(),
            ask: ask_tokens,
            claimant: None,
            //whitelisted_purchasers: whitelisted_addrs,
            whitelisted_buyer_one: wl_buyer_one,
            whitelisted_buyer_two: wl_buyer_two,
            whitelisted_buyer_three: wl_buyer_three
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_cw20_listing")
        .add_attribute("listing_id", &createlistingmsg.id)
        .add_attribute("creator", user_address.to_string()))
}

pub fn execute_create_listing_cw721(
    deps: DepsMut,
    user_wallet: &Addr,
    nft: Nft,
    createlistingmsg: CreateListingMsg,
) -> Result<Response, ContractError> {
    // Checking to make sure that listing_id isn't taken
    if (listingz().idx.id.item(deps.storage, createlistingmsg.id.clone())?).is_some() {
        return Err(ContractError::IdAlreadyExists {});
    }

    // let whitelisted_addrs =
    //     get_whitelisted_addresses(&deps, createlistingmsg.whitelisted_purchasers)?;

    let (wl_buyer_one, wl_buyer_two, wl_buyer_three) = 
        get_whitelisted_buyers(
            &deps, 
            createlistingmsg.whitelist_buyer_one,
            createlistingmsg.whitelist_buyer_two,
            createlistingmsg.whitelist_buyer_three
        )?;

    let ask_tokens = normalize_ask(createlistingmsg.ask);

    listingz().save(
        deps.storage,
        (user_wallet, createlistingmsg.id.clone()),
        &Listing {
            creator: user_wallet.clone(),
            id: createlistingmsg.id.clone(),
            finalized_time: None,
            expiration_time: None,
            status: Status::BeingPrepared,
            for_sale: genbal_from_nft(nft),
            ask: ask_tokens,
            claimant: None,
            //whitelisted_purchasers: whitelisted_addrs,
            whitelisted_buyer_one: wl_buyer_one,
            whitelisted_buyer_two: wl_buyer_two,
            whitelisted_buyer_three: wl_buyer_three
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_cw721_listing")
        .add_attribute("listing_id", &createlistingmsg.id)
        .add_attribute("creator", user_wallet.to_string()))
}

pub fn execute_change_ask(
    deps: DepsMut,
    user_sender: &Addr,
    listing_id: String,
    new_ask: GenericBalance,
) -> Result<Response, ContractError> {
    // Ensure listing exists, sender is owner, & get listing
    let Some(listing) = listingz().may_load(deps.storage, (user_sender, listing_id.clone()))? else {
        return Err(ContractError::NotFound {
            typ: "Listing".to_string(),
            id: listing_id
        });
    };

    // Ensure sender is owner
    if user_sender != &listing.creator {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure no finalized time
    if listing.finalized_time.is_some() {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Ensure being prepared
    if listing.status != Status::BeingPrepared {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Ensure no Claimant
    if listing.claimant.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    let new_ask_tokens = normalize_ask(new_ask);

    listingz().replace(
        deps.storage,
        (user_sender, listing_id.clone()),
        Some(&Listing {
            ask: new_ask_tokens,
            ..listing.clone()
        }),
        Some(&listing),
    )?;

    Ok(Response::new()
        .add_attribute("attribute", "change_listing_ask")
        .add_attribute("listing_id", &listing_id))
}

pub fn execute_add_funds_to_sale(
    deps: DepsMut,
    balance: Balance,
    user_sender: &Addr,
    listing_id: String,
) -> Result<Response, ContractError> {
    // Error if no funds sent
    if balance.is_empty() {
        return Err(ContractError::NoTokens {});
    }

    // Ensure listing exists & get listing
    let Some(listing) = listingz().may_load(deps.storage, (user_sender, listing_id.clone()))? else {
        return Err(ContractError::NotFound {
            typ: "Listing".to_string(),
            id: listing_id
        });
    };

    // Ensure sender is Creator
    if user_sender != &listing.creator {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure status is InPreperation
    if listing.status != Status::BeingPrepared {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Ensure no claimant <not already purchased>
    if listing.claimant.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    // Update old listing by adding tokens
    let new_listing = {
        let old_listing = listing.for_sale.clone();
        let mut x = listing.clone();
        x.for_sale.add_tokens(balance);
        if old_listing == x.for_sale {
            Err(ContractError::ErrorAdding("Tokens to Listing".to_string()))
        } else {
            Ok(x)
        }
    }?;

    listingz().replace(
        deps.storage,
        (user_sender, listing_id.clone()),
        Some(&new_listing),
        Some(&listing),
    )?;

    Ok(Response::new()
        .add_attribute("action", "add_funds_to_listing")
        .add_attribute("listing", &listing_id))
}

pub fn execute_add_to_sale_cw721(
    deps: DepsMut,
    user_wallet: &Addr,
    nft: Nft,
    listing_id: String,
) -> Result<Response, ContractError> {
    // Ensure listing exists & get listing
    let Some(old_listing) = listingz().may_load(deps.storage, (user_wallet, listing_id.clone()))? else {
        return Err(ContractError::NotFound {
            typ: "Listing".to_string(),
            id: listing_id,
        });
    };

    // Ensure sender is Creator
    if user_wallet != &old_listing.creator {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure status is InPreperation
    if old_listing.status != Status::BeingPrepared {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Ensure no claimant <not already purchased>
    if old_listing.claimant.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    // Create updated listing
    let new_listing = {
        let old = old_listing.for_sale.clone();
        let mut x = old_listing.clone();
        x.for_sale.add_nft(nft);
        if old == x.for_sale {
            Err(ContractError::ToDo {})
        } else {
            Ok(x)
        }
    }?;

    // Replace old listing with new listing
    listingz().replace(
        deps.storage,
        (user_wallet, listing_id),
        Some(&new_listing),
        Some(&old_listing),
    )?;

    Ok(Response::default())
}

pub fn execute_remove_listing(
    deps: DepsMut,
    user_sender: &Addr,
    listing_id: String,
) -> Result<Response, ContractError> {
    // Check listing exists & get listing
    let Some(listing) = listingz().may_load(deps.storage, (user_sender, listing_id.clone()))? else {
        return Err(ContractError::NotFound {
            typ: "Listing".to_string(),
            id: listing_id,
        });
    };

    // Only listing creator can remove listing
    if &listing.creator != user_sender {
        return Err(ContractError::Unauthorized {});
    }

    // Only can be removed if status is BeingPrepared
    if listing.status != Status::BeingPrepared {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Only can be removed if has not yet been purchased
    if listing.claimant.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    // Delete listing & send funds back to user
    let msgs = send_tokens_cosmos(&listing.creator, &listing.for_sale)?;

    listingz().remove(deps.storage, (user_sender, listing_id))?;

    Ok(Response::new().add_attribute("action", "remove_listing").add_messages(msgs))
}

pub fn execute_finalize(
    deps: DepsMut,
    env: &Env,
    sender: &Addr,
    listing_id: String,
    seconds: u64,
) -> Result<Response, ContractError> {
    // Ensure listing exists & get listing
    let Some(listing) = listingz().may_load(deps.storage, (sender, listing_id.clone()))? else {
        return Err(ContractError::NotFound {
            typ: "Listing".to_string(),
            id: listing_id,
        });
    };

    // Ensure sender is owner
    if sender != &listing.creator {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure no finalized time
    if listing.finalized_time.is_some() {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Ensure being prepared
    if listing.status != Status::BeingPrepared {
        return Err(ContractError::AlreadyFinalized {});
    }

    // Ensure no Claimant
    if listing.claimant.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    // max expiration is 1209600 seconds <14 days>
    // min expiration is 600 seconds <10 minutes>
    if !(600..=1_209_600).contains(&seconds) {
        return Err(ContractError::InvalidExpiration {});
    }

    let finalized_at = env.block.time;
    let expiration = env.block.time.plus_seconds(seconds);

    listingz().replace(
        deps.storage,
        (sender, listing_id.clone()),
        Some(&Listing {
            finalized_time: Some(finalized_at),
            expiration_time: Some(expiration),
            status: Status::FinalizedReady,
            ..listing.clone()
        }),
        Some(&listing),
    )?;

    Ok(Response::new()
        .add_attribute("action", "finalize")
        .add_attribute("listing_id", &listing_id)
        .add_attribute("expiration_seconds", expiration.to_string()))
}

pub fn execute_refund(
    deps: DepsMut,
    env: &Env,
    user_sender: &Addr,
    listing_id: String,
) -> Result<Response, ContractError> {
    // Check listing exists & get listing
    let Some(listing) = listingz().may_load(deps.storage, (user_sender, listing_id.clone()))? else {
        return Err(ContractError::NotFound {
            typ: "Listing".to_string(),
            id: listing_id,
        });
    };

    // Check sender is creator
    if user_sender.clone() != listing.creator {
        return Err(ContractError::Unauthorized {});
    }

    // If listing.claimant.is_some() then listing already purchased
    if listing.claimant.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    // Is listing.status == BeingPrepared
    if listing.status == Status::BeingPrepared {
        return Err(ContractError::Unauthorized {});
    }

    // Check if listing is expired
    match listing.expiration_time {
        None => {
            return Err(ContractError::Unauthorized {});
        }
        Some(timestamp) => {
            if env.block.time < timestamp {
                return Err(ContractError::NotExpired {
                    x: timestamp.eztime_string()?,
                });
            }
        }
    };

    // Checks pass, send refund & delete listing
    let refundee = listing.creator;
    let funds = listing.for_sale;
    let send_msgs = send_tokens_cosmos(&refundee, &funds)?;

    // Delete Listing
    listingz().remove(deps.storage, (user_sender, listing_id))?;

    Ok(Response::new().add_attribute("action", "refund").add_messages(send_msgs))
}

//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// Purchasing
//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
pub fn execute_buy_listing(
    deps: DepsMut,
    env: &Env,
    buyer: &Addr,
    listing_id: String,
    bucket_id: &str,
) -> Result<Response, ContractError> {
    // Get bucket (will error if no bucket found)
    let the_bucket = match BUCKETS.load(deps.storage, (buyer.clone(), bucket_id)) {
        Ok(buck) => buck,
        Err(_) => return Err(ContractError::LoadBucketError {}),
    };

    // Check listing exists & get the_listing
    let Some((_pk, the_listing)): Option<(_, Listing)> = listingz().idx.id.item(deps.storage, listing_id.clone())? else {
        return Err(ContractError::NotFound { typ: "Listing".to_string(), id: listing_id });
    };

    // Check that sender is bucket owner (redundant check)
    if buyer != &the_bucket.owner {
        return Err(ContractError::Unauthorized {});
    }
    // Check that bucket contains required purchase price
    if the_bucket.funds != the_listing.ask {
        return Err(ContractError::FundsSentNotFundsAsked {
            which: format!("Bucket ID: {}", bucket_id),
        });
    }
    // Check that listing is ready for purchase
    if the_listing.status != Status::FinalizedReady {
        return Err(ContractError::NotPurchasable {});
    }
    // Check that the user buying is whitelisted
    check_buyer_whitelisted(buyer, &the_listing)?;
    // if let Some(whitelist) = the_listing.whitelisted_purchasers.clone() {
    //     if !whitelist.contains(buyer) {
    //         return Err(ContractError::NotWhitelisted {});
    //     }
    // }

    // Check that there's no existing claimant on listing
    if the_listing.claimant.is_some() {
        return Err(ContractError::NotPurchasable {});
    }
    // Check that listing isn't expired
    if let Some(exp) = the_listing.expiration_time {
        if env.block.time > exp {
            return Err(ContractError::Expired {});
        }
    }

    // Delete Old Listing -> Save new listing with listing_buyer in key & creator
    listingz().remove(deps.storage, (&the_listing.creator, listing_id.clone()))?;
    listingz().save(
        deps.storage,
        (buyer, listing_id.clone()),
        &Listing {
            creator: buyer.clone(),
            claimant: Some(buyer.clone()),
            status: Status::Closed,
            ..the_listing
        },
    )?;

    // Delete Old Bucket -> Save new Bucket with listing_seller in key & owner
    BUCKETS.remove(deps.storage, (buyer.clone(), bucket_id));
    BUCKETS.save(
        deps.storage,
        (the_listing.creator.clone(), bucket_id),
        &Bucket {
            owner: the_listing.creator,
            ..the_bucket
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "buy_listing")
        .add_attribute("bucket_used", bucket_id)
        .add_attribute("listing_purchased:", &listing_id))
}

pub fn execute_withdraw_purchased(
    deps: DepsMut,
    withdrawer: &Addr,
    listing_id: String,
) -> Result<Response, ContractError> {
    // Get listing
    let Some((_pk, the_listing)): Option<(_, Listing)> = listingz().idx.id.item(deps.storage, listing_id.clone())? else {
        return Err(ContractError::NotFound { typ: "Listing".to_string(), id: listing_id });
    };

    // Check and pull out claimant
    let listing_claimr = the_listing.claimant.ok_or(ContractError::Unauthorized {})?;

    // Check that withdrawer is the claimant
    if withdrawer != &listing_claimr {
        return Err(ContractError::Unauthorized {});
    };

    // Check that status is Closed
    if the_listing.status != Status::Closed {
        return Err(ContractError::Unauthorized {});
    };

    // Delete Listing
    listingz().remove(deps.storage, (&listing_claimr, listing_id.clone()))?;

    // default listing response
    let res: Response = Response::new()
        .add_attribute("action", "withdraw_purchased")
        .add_attribute("listing_id", listing_id);

    if let Some((fee_msg, gbal)) =
        calc_fee(&the_listing.for_sale).map_err(|_foo| ContractError::FeeCalc)?
    {
        let user_msgs = send_tokens_cosmos(&listing_claimr, &gbal)?;
        Ok(res.add_message(fee_msg).add_messages(user_msgs))
    } else {
        let user_msgs = send_tokens_cosmos(&listing_claimr, &the_listing.for_sale)?;
        Ok(res.add_messages(user_msgs))
    }
}
