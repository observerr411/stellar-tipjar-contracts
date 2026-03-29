//! Milestone NFT minting and transfer logic.

use soroban_sdk::{panic_with_error, Address, Env, Map, String, Vec};

use crate::{DataKey, TipJarError};

use super::{metadata::build_metadata, MilestoneNft};

/// Default milestone threshold used when a creator has not configured one.
pub const DEFAULT_MILESTONE_STEP: i128 = 1_000;

/// Configures a creator-specific milestone threshold.
pub fn set_milestone_step(env: &Env, creator: &Address, milestone_step: i128) {
    if milestone_step <= 0 {
        panic_with_error!(env, TipJarError::InvalidGoalAmount);
    }

    env.storage()
        .persistent()
        .set(&DataKey::CreatorMilestoneStep(creator.clone()), &milestone_step);
}

/// Reads milestone threshold for a creator, falling back to default.
pub fn get_milestone_step(env: &Env, creator: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::CreatorMilestoneStep(creator.clone()))
        .unwrap_or(DEFAULT_MILESTONE_STEP)
}

/// Records a tip and mints NFTs for all newly crossed milestone levels.
pub fn record_tip_and_maybe_mint(
    env: &Env,
    sender: &Address,
    creator: &Address,
    amount: i128,
    base_metadata: &Map<String, String>,
) -> Vec<u64> {
    if amount <= 0 {
        panic_with_error!(env, TipJarError::InvalidAmount);
    }

    let previous_total = get_creator_total(env, creator);
    let new_total = previous_total + amount;
    env.storage()
        .persistent()
        .set(&DataKey::CreatorMilestoneTipTotal(creator.clone()), &new_total);

    let step = get_milestone_step(env, creator);
    let prev_level = previous_total / step;
    let new_level = new_total / step;

    let mut minted_ids = Vec::new(env);
    if new_level <= prev_level {
        return minted_ids;
    }

    let mut level = prev_level + 1;
    while level <= new_level {
        let token_id = mint_single(env, sender, creator, level as u64, new_total, base_metadata);
        minted_ids.push_back(token_id);
        level += 1;
    }

    minted_ids
}

/// Transfers ownership of an NFT token.
pub fn transfer_nft(env: &Env, from: &Address, to: &Address, token_id: u64) {
    let mut nft = get_nft_or_panic(env, token_id);
    if nft.owner != *from {
        panic_with_error!(env, TipJarError::NotNftOwner);
    }

    nft.owner = to.clone();
    env.storage()
        .persistent()
        .set(&DataKey::NftToken(token_id), &nft);

    remove_owner_token(env, from, token_id);
    add_owner_token(env, to, token_id);
}

/// Returns NFT token data if present.
pub fn get_nft(env: &Env, token_id: u64) -> Option<MilestoneNft> {
    env.storage().persistent().get(&DataKey::NftToken(token_id))
}

/// Returns all token IDs owned by an address.
pub fn get_owner_tokens(env: &Env, owner: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::NftOwnerTokens(owner.clone()))
        .unwrap_or(Vec::new(env))
}

fn get_creator_total(env: &Env, creator: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::CreatorMilestoneTipTotal(creator.clone()))
        .unwrap_or(0)
}

fn mint_single(
    env: &Env,
    owner: &Address,
    creator: &Address,
    milestone_level: u64,
    cumulative_tips: i128,
    base_metadata: &Map<String, String>,
) -> u64 {
    let token_id = next_token_id(env);
    let metadata = build_metadata(env, creator, milestone_level, cumulative_tips, base_metadata);

    let nft = MilestoneNft {
        token_id,
        owner: owner.clone(),
        creator: creator.clone(),
        milestone_level,
        cumulative_tips,
        metadata,
        minted_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::NftToken(token_id), &nft);
    add_owner_token(env, owner, token_id);

    env.events().publish(
        (soroban_sdk::symbol_short!("nftmint"), creator.clone()),
        (owner.clone(), token_id, milestone_level, cumulative_tips),
    );

    token_id
}

fn next_token_id(env: &Env) -> u64 {
    let current = env
        .storage()
        .persistent()
        .get::<_, u64>(&DataKey::NftCounter)
        .unwrap_or(0);
    let next = current + 1;
    env.storage().persistent().set(&DataKey::NftCounter, &next);
    next
}

fn get_nft_or_panic(env: &Env, token_id: u64) -> MilestoneNft {
    get_nft(env, token_id).unwrap_or_else(|| panic_with_error!(env, TipJarError::NftNotFound))
}

fn add_owner_token(env: &Env, owner: &Address, token_id: u64) {
    let mut owned = get_owner_tokens(env, owner);
    owned.push_back(token_id);
    env.storage()
        .persistent()
        .set(&DataKey::NftOwnerTokens(owner.clone()), &owned);
}

fn remove_owner_token(env: &Env, owner: &Address, token_id: u64) {
    let owned = get_owner_tokens(env, owner);
    let mut updated = Vec::new(env);

    for existing in owned.iter() {
        if existing != token_id {
            updated.push_back(existing);
        }
    }

    env.storage()
        .persistent()
        .set(&DataKey::NftOwnerTokens(owner.clone()), &updated);
}

#[cfg(test)]
mod tests {
    extern crate std;

    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, Map, String};

    use super::{get_nft, get_owner_tokens, record_tip_and_maybe_mint, set_milestone_step, transfer_nft};

    fn base_metadata(env: &Env) -> Map<String, String> {
        let mut map = Map::new(env);
        map.set(
            String::from_str(env, "name"),
            String::from_str(env, "Milestone Medal"),
        );
        map
    }

    #[test]
    fn mints_when_milestone_crossed() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let creator = Address::generate(&env);

        set_milestone_step(&env, &creator, 100);

        let ids_first = record_tip_and_maybe_mint(&env, &sender, &creator, 60, &base_metadata(&env));
        assert_eq!(ids_first.len(), 0);

        let ids_second = record_tip_and_maybe_mint(&env, &sender, &creator, 50, &base_metadata(&env));
        assert_eq!(ids_second.len(), 1);

        let token_id = ids_second.get(0).unwrap();
        let nft = get_nft(&env, token_id).unwrap();
        assert_eq!(nft.milestone_level, 1);
    }

    #[test]
    fn supports_metadata_and_transfer() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let creator = Address::generate(&env);
        let recipient = Address::generate(&env);

        set_milestone_step(&env, &creator, 10);

        let minted = record_tip_and_maybe_mint(&env, &sender, &creator, 10, &base_metadata(&env));
        let token_id = minted.get(0).unwrap();

        let before = get_nft(&env, token_id).unwrap();
        assert_eq!(before.owner, sender);
        assert!(before
            .metadata
            .contains_key(String::from_str(&env, "milestone_level")));

        transfer_nft(&env, &sender, &recipient, token_id);

        let after = get_nft(&env, token_id).unwrap();
        assert_eq!(after.owner, recipient);
        assert_eq!(get_owner_tokens(&env, &sender).len(), 0);
        assert_eq!(get_owner_tokens(&env, &recipient).len(), 1);
    }

    #[test]
    fn token_ids_are_unique() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let creator = Address::generate(&env);

        set_milestone_step(&env, &creator, 50);

        let one = record_tip_and_maybe_mint(&env, &sender, &creator, 50, &base_metadata(&env));
        let two = record_tip_and_maybe_mint(&env, &sender, &creator, 50, &base_metadata(&env));

        assert_eq!(one.get(0).unwrap(), 1);
        assert_eq!(two.get(0).unwrap(), 2);
    }
}
