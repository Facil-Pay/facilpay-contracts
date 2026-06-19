#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

#[test]
fn test_two_level_hierarchy_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    // Create root parent escrow (level 0)
    let parent_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &2000_u64, &0_u64, &0_u64, &false,
    );

    // Create child escrow (level 1)
    let child_id =
        client.create_child_escrow(&admin, &parent_id, &500_i128, &token, &customer, &merchant);

    // Verify parent release is blocked since child is unresolved (Locked)
    let release_res = client.try_release_escrow(&admin, &parent_id, &false);
    assert_eq!(release_res, Err(Ok(Error::ChildrenNotResolved)));

    // Advance ledger timestamp so child can be released without early release check
    env.ledger().set_timestamp(2000);

    // Resolve child (release it)
    client.release_escrow(&admin, &child_id, &false);

    // Now parent should be able to release successfully (once the ledger timestamp passes the release timestamp)
    env.ledger().set_timestamp(2001);
    let release_res = client.try_release_escrow(&admin, &parent_id, &false);
    assert!(release_res.is_ok());
}

#[test]
fn test_depth_limit_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    // Root (level 0)
    let root_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );

    // Child (level 1)
    let lvl1_id =
        client.create_child_escrow(&admin, &root_id, &500_i128, &token, &customer, &merchant);

    // Grandchild (level 2)
    let lvl2_id =
        client.create_child_escrow(&admin, &lvl1_id, &250_i128, &token, &customer, &merchant);

    // Great-grandchild (level 3)
    let lvl3_id =
        client.create_child_escrow(&admin, &lvl2_id, &100_i128, &token, &customer, &merchant);

    // Creating under level 3 (would be level 4) should fail with MaxHierarchyDepth
    let lvl4_res =
        client.try_create_child_escrow(&admin, &lvl3_id, &50_i128, &token, &customer, &merchant);
    assert_eq!(lvl4_res, Err(Ok(Error::MaxHierarchyDepth)));
}

#[test]
fn test_get_escrow_hierarchy() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    let root_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );

    let child_1 =
        client.create_child_escrow(&admin, &root_id, &500_i128, &token, &customer, &merchant);

    let child_2 =
        client.create_child_escrow(&admin, &root_id, &300_i128, &token, &customer, &merchant);

    let grandchild_1 =
        client.create_child_escrow(&admin, &child_1, &100_i128, &token, &customer, &merchant);

    let hierarchy = client.get_escrow_hierarchy(&root_id);
    assert_eq!(hierarchy.len(), 4);

    // Root node checks
    let root_node = hierarchy.get(0).unwrap();
    assert_eq!(root_node.escrow_id, root_id);
    assert_eq!(root_node.parent_id, None);
    assert_eq!(root_node.depth, 0);
    assert_eq!(root_node.children.len(), 2);
    assert_eq!(root_node.children.get(0).unwrap(), child_1);
    assert_eq!(root_node.children.get(1).unwrap(), child_2);

    // Child 1 node checks
    let child1_node = hierarchy.get(1).unwrap();
    assert_eq!(child1_node.escrow_id, child_1);
    assert_eq!(child1_node.parent_id, Some(root_id));
    assert_eq!(child1_node.depth, 1);
    assert_eq!(child1_node.children.len(), 1);
    assert_eq!(child1_node.children.get(0).unwrap(), grandchild_1);
}

#[test]
fn test_create_child_escrow_validation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unauthorized_caller = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    // Non-admin call fails with NotAnAdmin
    let res = client.try_create_child_escrow(
        &unauthorized_caller,
        &1_u64,
        &100_i128,
        &token,
        &customer,
        &merchant,
    );
    assert_eq!(res, Err(Ok(Error::NotAnAdmin)));

    // Non-existent parent fails with ParentEscrowNotFound
    let res2 =
        client.try_create_child_escrow(&admin, &999_u64, &100_i128, &token, &customer, &merchant);
    assert_eq!(res2, Err(Ok(Error::ParentEscrowNotFound)));
}
