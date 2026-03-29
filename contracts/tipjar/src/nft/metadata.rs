//! Metadata helpers for milestone NFTs.

extern crate alloc;

use alloc::string::ToString;

use soroban_sdk::{Address, Env, Map, String};

/// Merges caller-provided metadata with standardized milestone fields.
pub fn build_metadata(
    env: &Env,
    creator: &Address,
    milestone_level: u64,
    cumulative_tips: i128,
    base_metadata: &Map<String, String>,
) -> Map<String, String> {
    let mut metadata = base_metadata.clone();

    metadata.set(
        String::from_str(env, "standard"),
        String::from_str(env, "tipjar-milestone-nft-v1"),
    );
    metadata.set(
        String::from_str(env, "creator"),
        creator.to_string(),
    );

    let milestone_level_text = milestone_level.to_string();
    metadata.set(
        String::from_str(env, "milestone_level"),
        String::from_str(env, milestone_level_text.as_str()),
    );

    let cumulative_tips_text = cumulative_tips.to_string();
    metadata.set(
        String::from_str(env, "cumulative_tips"),
        String::from_str(env, cumulative_tips_text.as_str()),
    );

    metadata
}
