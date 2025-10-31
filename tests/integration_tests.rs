use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use vault_litesvm::{instruction::VaultIx, states::VaultAccount};

fn create_deposit_instruction(
    program_id: &Pubkey,
    user: &Pubkey,
    vault_pda: &Pubkey,
    amount: u64,
) -> Instruction {
    let instruction_data = VaultIx::Deposit { amount };

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new_readonly(
                Pubkey::from_str_const("11111111111111111111111111111111"),
                false,
            ),
        ],
        data: borsh::to_vec(&instruction_data).unwrap(),
    }
}

fn create_withdraw_instruction(
    program_id: &Pubkey,
    user: &Pubkey,
    vault_pda: &Pubkey,
    amount: u64,
) -> Instruction {
    let instruction_data = VaultIx::Withdraw { amount };

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*vault_pda, false),
        ],
        data: borsh::to_vec(&instruction_data).unwrap(),
    }
}

#[test]
fn test_deposit_and_withdraw() {
    let mut svm = LiteSVM::new();

    let program_id = Pubkey::new_unique();

    let program_account = Account {
        lamports: 1_000_000_000,
        data: vec![],
        owner: solana_sdk::bpf_loader::id(),
        executable: true,
        rent_epoch: 0,
    };
    let _ = svm.set_account(program_id, program_account);

    let user = Keypair::new();
    let user_pubkey = user.pubkey();

    let user_account = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: Pubkey::from_str_const("11111111111111111111111111111111"),
        executable: false,
        rent_epoch: 0,
    };
    let _ = svm.set_account(user_pubkey, user_account);

    let (vault_pda, _bump) =
        Pubkey::find_program_address(&[VaultAccount::SEED, user_pubkey.as_ref()], &program_id);

    println!("Program ID: {}", program_id);
    println!("User: {}", user_pubkey);
    println!("Vault PDA: {}", vault_pda);

    println!("\n--- Test 1: Deposit 1 SOL ---");
    let deposit_amount = 1_000_000_000; // 1 SOL

    let deposit_ix =
        create_deposit_instruction(&program_id, &user_pubkey, &vault_pda, deposit_amount);

    let recent_blockhash = svm.latest_blockhash();
    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user_pubkey),
        &[&user],
        recent_blockhash,
    );

    let result = svm.send_transaction(deposit_tx);
    println!("Deposit result: {:?}", result);
    assert!(result.is_ok(), "Deposit should succeed");

    let vault_account = svm.get_account(&vault_pda);
    assert!(vault_account.is_some(), "Vault account should exist");

    let vault_data = vault_account.unwrap();
    let vault_state: VaultAccount =
        borsh::BorshDeserialize::try_from_slice(&vault_data.data).unwrap();

    println!("Vault owner: {}", vault_state.owner);
    println!("Vault balance: {}", vault_state.balance);
    println!("Vault bump: {}", vault_state.bump);

    assert_eq!(vault_state.owner, user_pubkey, "Vault owner should be user");
    assert_eq!(
        vault_state.balance, deposit_amount,
        "Vault balance should equal deposit amount"
    );

    println!("\n--- Test 2: Deposit another 0.5 SOL ---");
    let second_deposit = 500_000_000;

    let deposit_ix2 =
        create_deposit_instruction(&program_id, &user_pubkey, &vault_pda, second_deposit);

    let recent_blockhash = svm.latest_blockhash();
    let deposit_tx2 = Transaction::new_signed_with_payer(
        &[deposit_ix2],
        Some(&user_pubkey),
        &[&user],
        recent_blockhash,
    );

    let result = svm.send_transaction(deposit_tx2);
    println!("Second deposit result: {:?}", result);
    assert!(result.is_ok(), "Second deposit should succeed");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    let vault_state: VaultAccount =
        borsh::BorshDeserialize::try_from_slice(&vault_account.data).unwrap();

    let expected_balance = deposit_amount + second_deposit;
    println!("New vault balance: {}", vault_state.balance);
    assert_eq!(
        vault_state.balance, expected_balance,
        "Vault balance should be 1.5 SOL"
    );

    println!("\n--- Test 3: Withdraw 0.5 SOL ---");
    let withdraw_amount = 500_000_000;

    let user_balance_before = svm.get_account(&user_pubkey).unwrap().lamports;
    println!("User balance before withdraw: {}", user_balance_before);

    let withdraw_ix =
        create_withdraw_instruction(&program_id, &user_pubkey, &vault_pda, withdraw_amount);

    let recent_blockhash = svm.latest_blockhash();
    let withdraw_tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&user_pubkey),
        &[&user],
        recent_blockhash,
    );

    let result = svm.send_transaction(withdraw_tx);
    println!("Withdraw result: {:?}", result);
    assert!(result.is_ok(), "Withdraw should succeed");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    let vault_state: VaultAccount =
        borsh::BorshDeserialize::try_from_slice(&vault_account.data).unwrap();

    let expected_balance = deposit_amount + second_deposit - withdraw_amount;
    println!("Vault balance after withdraw: {}", vault_state.balance);
    assert_eq!(
        vault_state.balance, expected_balance,
        "Vault balance should be 1 SOL"
    );

    let user_balance_after = svm.get_account(&user_pubkey).unwrap().lamports;
    println!("User balance after withdraw: {}", user_balance_after);

    println!("\n--- Test 4: Try to withdraw more than balance (should fail) ---");
    let excessive_withdraw = 2_000_000_000;

    let withdraw_ix =
        create_withdraw_instruction(&program_id, &user_pubkey, &vault_pda, excessive_withdraw);

    let recent_blockhash = svm.latest_blockhash();
    let withdraw_tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&user_pubkey),
        &[&user],
        recent_blockhash,
    );

    let result = svm.send_transaction(withdraw_tx);
    println!("Excessive withdraw result: {:?}", result);
    assert!(result.is_err(), "Excessive withdraw should fail");

    println!("\nAll tests passed!");
}

#[test]
fn test_unauthorized_withdrawal() {
    // Initialize LiteSVM
    let mut svm = LiteSVM::new();

    let program_id = Pubkey::new_unique();
    let program_account = Account {
        lamports: 1_000_000_000,
        data: vec![],
        owner: solana_sdk::bpf_loader::id(),
        executable: true,
        rent_epoch: 0,
    };
    let _ = svm.set_account(program_id, program_account);

    let user1 = Keypair::new();
    let user2 = Keypair::new();

    let user1_pubkey = user1.pubkey();
    let user2_pubkey = user2.pubkey();

    let user1_account = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: Pubkey::from_str_const("11111111111111111111111111111111"),
        executable: false,
        rent_epoch: 0,
    };
    let user2_account = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: Pubkey::from_str_const("11111111111111111111111111111111"),
        executable: false,
        rent_epoch: 0,
    };
    let _ = svm.set_account(user1_pubkey, user1_account);
    let _ = svm.set_account(user2_pubkey, user2_account);

    // User1's vault PDA
    let (vault_pda_user1, _) =
        Pubkey::find_program_address(&[VaultAccount::SEED, user1_pubkey.as_ref()], &program_id);

    println!("\n--- Test: Unauthorized Withdrawal ---");
    println!("User1: {}", user1_pubkey);
    println!("User2: {}", user2_pubkey);
    println!("User1 Vault PDA: {}", vault_pda_user1);

    let deposit_amount = 1_000_000_000;
    let deposit_ix =
        create_deposit_instruction(&program_id, &user1_pubkey, &vault_pda_user1, deposit_amount);

    let recent_blockhash = svm.latest_blockhash();
    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user1_pubkey),
        &[&user1],
        recent_blockhash,
    );

    let result = svm.send_transaction(deposit_tx);
    assert!(result.is_ok(), "User1 deposit should succeed");
    println!("User1 deposited 1 SOL");

    let withdraw_ix =
        create_withdraw_instruction(&program_id, &user2_pubkey, &vault_pda_user1, 500_000_000);

    let recent_blockhash = svm.latest_blockhash();
    let withdraw_tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&user2_pubkey),
        &[&user2],
        recent_blockhash,
    );

    let result = svm.send_transaction(withdraw_tx);
    println!("User2 withdrawal attempt result: {:?}", result);
    assert!(
        result.is_err(),
        "User2 should not be able to withdraw from User1's vault"
    );

    println!("Unauthorized withdrawal correctly blocked!");
}

#[test]
fn test_multiple_user_vaults() {
    let mut svm = LiteSVM::new();

    let program_id = Pubkey::new_unique();
    let program_account = Account {
        lamports: 1_000_000_000,
        data: vec![],
        owner: solana_sdk::bpf_loader::id(),
        executable: true,
        rent_epoch: 0,
    };
    let _ = svm.set_account(program_id, program_account);

    let users: Vec<Keypair> = (0..3).map(|_| Keypair::new()).collect();

    println!("\n--- Test: Multiple User Vaults ---");

    for (i, user) in users.iter().enumerate() {
        let user_pubkey = user.pubkey();

        let user_account = Account {
            lamports: 10_000_000_000,
            data: vec![],
            owner: Pubkey::from_str_const("11111111111111111111111111111111"),
            executable: false,
            rent_epoch: 0,
        };
        let _ = svm.set_account(user_pubkey, user_account);

        let (vault_pda, _) =
            Pubkey::find_program_address(&[VaultAccount::SEED, user_pubkey.as_ref()], &program_id);

        println!("\nUser {}: {}", i + 1, user_pubkey);
        println!("Vault PDA: {}", vault_pda);

        let deposit_amount = (i as u64 + 1) * 500_000_000; // 0.5, 1, 1.5 SOL

        let deposit_ix =
            create_deposit_instruction(&program_id, &user_pubkey, &vault_pda, deposit_amount);

        let recent_blockhash = svm.latest_blockhash();
        let deposit_tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user_pubkey),
            &[user],
            recent_blockhash,
        );

        let result = svm.send_transaction(deposit_tx);
        assert!(result.is_ok(), "User {} deposit should succeed", i + 1);

        let vault_account = svm.get_account(&vault_pda).unwrap();
        let vault_state: VaultAccount =
            borsh::BorshDeserialize::try_from_slice(&vault_account.data).unwrap();

        assert_eq!(vault_state.balance, deposit_amount);
        assert_eq!(vault_state.owner, user_pubkey);

        println!("User {} deposited {} lamports", i + 1, deposit_amount);
    }

    println!("\nAll users have unique vaults with correct balances!");
}
