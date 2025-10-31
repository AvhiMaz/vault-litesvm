use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VaultIx {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
}
