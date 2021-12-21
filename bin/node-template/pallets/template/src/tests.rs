use super::*;
use frame_support::{assert_ok};
use mock::*;
use sp_core::Pair;
use sp_core::{
	offchain::{testing, OffchainWorkerExt, TransactionPoolExt, OffchainDbExt}
};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use std::sync::Arc;

#[test]
fn iris_initial_state() {
	new_test_ext().execute_with(|| {
		// Given: The node is initialized at block 0
		// When: I query runtime storagey
		let data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		// Then: Runtime storage is empty
		assert_eq!(len, 0);
	});
}

#[test]
fn iris_ipfs_add_bytes_works_for_valid_value() {
	// Given: I am a valid node with a positive balance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let multiaddr_vec = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp".as_bytes().to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let name: Vec<u8> = "test.txt".as_bytes().to_vec();
	let cost = 1;
	let id = 1;
	let balance = 1;

	// 
	let expected_data_command = crate::DataCommand::AddBytes(
		OpaqueMultiaddr(multiaddr_vec.clone()),
		cid_vec.clone(),
		p.clone().public(),
		name.clone(),
		id.clone(),
		balance.clone(),
	);

	new_test_ext_funded(p.clone()).execute_with(|| {
		// WHEN: I invoke the create_storage_assets extrinsic
		assert_ok!(Iris::create_storage_asset(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			multiaddr_vec.clone(),
			cid_vec.clone(),
			name.clone(),
			id.clone(),
			balance.clone(),
		));

		// THEN: There is a single DataCommand::AddBytes in the DataQueue
		let mut data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		assert_eq!(len, 1);
		let actual_data_command = data_queue.pop();
		assert_eq!(actual_data_command, Some(expected_data_command));
	});
}

#[test]
fn iris_request_data_works_for_valid_values() {
	// GIVEN: I am a valid Iris node with a positive balance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();

	let expected_data_command = crate::DataCommand::CatBytes(
		p.clone().public(),
		cid_vec.clone(),
		p.clone().public(),
	);
	new_test_ext_funded(p.clone()).execute_with(|| {
		// WHEN: I invoke the request_data extrinsic
		assert_ok!(Iris::request_data(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
		));

		// THEN: There should be a single DataCommand::CatBytes in the DataQueue
		let mut data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		assert_eq!(len, 1);
		let actual_data_command = data_queue.pop();
		assert_eq!(actual_data_command, Some(expected_data_command));
	});
}

#[test]
fn iris_submit_ipfs_add_results_works_for_valid_values() {
	// GIVEN: I am a valid Iris node with a positive valance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let id = 1;
	let balance = 1;

	new_test_ext_funded(p.clone()).execute_with(|| {
		// WHEN: I invoke the submit_ipfs_add_results extrinsic
		assert_ok!(Iris::submit_ipfs_add_results(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			id.clone(),
			balance.clone(),
		));

		// THEN: a new asset class is created
		// AND: A new entry is added to the AssetClassOwnership StorageDoubleMap
		let admin_asset_class_id = crate::AssetClassOwnership::<Test>::get(p.clone().public(), cid_vec.clone());
		assert_eq!(admin_asset_class_id, id.clone());
	});
}

#[test]
fn iris_mint_tickets_works_for_valid_values() {
	// GIVEN: I am a valid Iris node with a positive valance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let balance = 1;
	let id = 1;

	new_test_ext_funded(p.clone()).execute_with(|| {
		// AND: I create an owned asset class
		assert_ok!(Iris::submit_ipfs_add_results(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			id.clone(),
			balance.clone(),
		));
		// WHEN: I invoke the mint_tickets extrinsic
		assert_ok!(Iris::mint_tickets(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			balance.clone(),
		));
		// THEN: new assets are created and awarded to the benficiary
		// AND: A new entry is added to the AssetAccess StorageDoubleMap
		let asset_class_owner = crate::AssetAccess::<Test>::get(p.clone().public(), cid_vec.clone());
		assert_eq!(asset_class_owner, p.clone().public())
	});
}

#[test]
fn iris_submit_rpc_ready_works_for_valid_values() {
	let (p, _) = sp_core::sr25519::Pair::generate();
	new_test_ext_funded(p.clone()).execute_with(|| {
		assert_ok!(Iris::submit_rpc_ready(
			Origin::signed(p.clone().public()),
			p.clone().public(),
		));
	});
}

// test OCW functionality
// can add bytes to network

#[test]
fn iris_can_add_bytes_to_ipfs() {
	let (p, _) = sp_core::sr25519::Pair::generate();
	let (offchain, state) = testing::TestOffchainExt::new();
	let (pool, _) = testing::TestTransactionPoolExt::new();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(
		&keystore,
		crate::KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.unwrap();

	let mut t = new_test_ext_funded(p.clone());
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let multiaddr_vec = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp".as_bytes().to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let bytes = "hello test".as_bytes().to_vec();
	let name: Vec<u8> = "test.txt".as_bytes().to_vec();
	let id = 1;
	let balance = 1;
	// mock IPFS calls
	{	
		let mut state = state.write();
		// connect to external node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// fetch data
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			id: sp_core::offchain::IpfsRequestId(0),
			response: Some(IpfsResponse::CatBytes(bytes.clone())),
			..Default::default()
		});
		// disconnect from the external node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// add bytes to your local node 
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::AddBytes(cid_vec.clone())),
			..Default::default()
		});
	}

	t.execute_with(|| {
		// WHEN: I invoke the create_storage_assets extrinsic
		assert_ok!(Iris::create_storage_asset(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			multiaddr_vec.clone(),
			cid_vec.clone(),
			name.clone(),
			id.clone(),
			balance.clone(),
		));
		// THEN: the offchain worker adds data to IPFS
		assert_ok!(Iris::handle_data_requests());
	});
}

// fn offchain_db() -> Db<LocalStorage> {
// 	Db::new(LocalStorage::new_test())
// }

// can fetch bytes and add to offchain storage
#[test]
fn iris_can_fetch_bytes_and_add_to_offchain_storage() {
	let (p, _) = sp_core::sr25519::Pair::generate();
	let (offchain, state) = testing::TestOffchainExt::new();
	let (pool, _) = testing::TestTransactionPoolExt::new();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(
		&keystore,
		crate::KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.unwrap();

	let mut t = new_test_ext_funded(p.clone());
	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(OffchainDbExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let multiaddr_vec = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp".as_bytes().to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let bytes = "hello test".as_bytes().to_vec();
	let name: Vec<u8> = "test.txt".as_bytes().to_vec();
	let id = 1;
	let balance = 1;
	// mock IPFS calls
	{	
		let mut state = state.write();
		// connect to external node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// fetch data
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			id: sp_core::offchain::IpfsRequestId(0),
			response: Some(IpfsResponse::CatBytes(bytes.clone())),
			..Default::default()
		});
		// disconnect from the external node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// add bytes to your local node 
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::AddBytes(cid_vec.clone())),
			..Default::default()
		});
		// fetch data
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			id: sp_core::offchain::IpfsRequestId(0),
			response: Some(IpfsResponse::CatBytes(bytes.clone())),
			..Default::default()
		});
	}

	t.execute_with(|| {
		// WHEN: I invoke the create_storage_assets extrinsic
		assert_ok!(Iris::create_storage_asset(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			multiaddr_vec.clone(),
			cid_vec.clone(),
			name.clone(),
			id.clone(),
			balance.clone(),
		));
		// AND: I create an owned asset class
		assert_ok!(Iris::submit_ipfs_add_results(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			id.clone(),
			balance.clone(),
		));
		// AND: I invoke the mint_tickets extrinsic
		assert_ok!(Iris::mint_tickets(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			balance.clone(),
		));
		// AND: I request the owned content from iris
		assert_ok!(Iris::request_data(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
		));
		// THEN: the offchain worker adds data to IPFS
		assert_ok!(Iris::handle_data_requests());
		// AND: The data is available in local offchain storage
	});	
}