use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct VaultAccount {
    pub owner: Pubkey,
    pub balance: u64,
    pub bump: u8,
}

impl VaultAccount {
    pub const LEN: usize = 32 + 8 + 1;

    pub const SEED: &'static str = "vault_account";
}
