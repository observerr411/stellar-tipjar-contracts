//! Milestone NFT module.
//!
//! Provides data types and helpers for commemorative NFTs minted when creator
//! tipping milestones are reached.

pub mod metadata;
pub mod minting;

use soroban_sdk::{contracttype, Address, Map, String};

/// Commemorative NFT minted when milestone thresholds are reached.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneNft {
    /// Globally unique token ID.
    pub token_id: u64,
    /// Current owner of the NFT.
    pub owner: Address,
    /// Creator this milestone belongs to.
    pub creator: Address,
    /// Milestone level reached (1-based).
    pub milestone_level: u64,
    /// Creator cumulative tips at mint time.
    pub cumulative_tips: i128,
    /// Arbitrary NFT metadata.
    pub metadata: Map<String, String>,
    /// Ledger timestamp at mint.
    pub minted_at: u64,
}
