#![cfg(test)]

use super::*;
use mock::{Runtime as Test, RuntimeEvent as Event, RuntimeOrigin as Origin, *};

use crate::runner::handler::Handler;
use frame_support::{assert_noop, assert_ok};
use sp_core::{
    bytes::{from_hex, to_hex},
    H160,
};
use sp_runtime::{traits::BadOrigin, AccountId32};
use std::str::FromStr;

#[test]
fn fail_call_return_ok() {
    new_test_ext().execute_with(|| {
        let mut data = [0u8; 32];
        data[0..4].copy_from_slice(b"evm:");
        let signer: AccountId32 = AccountId32::from(data).into();

        let origin = Origin::signed(signer);
        assert_ok!(EVM::call(
            origin.clone(),
            contract_a(),
            Vec::new(),
            0,
            1000000,
            0
        ));
        assert_ok!(EVM::call(origin, contract_b(), Vec::new(), 0, 1000000, 0));
    });
}

#[test]
fn should_calculate_contract_address() {
    new_test_ext().execute_with(|| {
        let addr = H160::from_str("bec02ff0cbf20042a37d964c33e89f1a2be7f068").unwrap();

        assert_eq!(
            Handler::<Test>::create_address(evm::CreateScheme::Legacy { caller: addr }),
            Ok(H160::from_str("d654cB21c05cb14895baae28159b1107e9DbD6E4").unwrap())
        );

        Handler::<Test>::inc_nonce(addr);
        assert_eq!(
            Handler::<Test>::create_address(evm::CreateScheme::Legacy { caller: addr }),
            Ok(H160::from_str("97784910F057B07bFE317b0552AE23eF34644Aed").unwrap())
        );

        Handler::<Test>::inc_nonce(addr);
        assert_eq!(
            Handler::<Test>::create_address(evm::CreateScheme::Legacy { caller: addr }),
            Ok(H160::from_str("82155a21E0Ccaee9D4239a582EB2fDAC1D9237c5").unwrap())
        );
    });
}

#[test]
fn should_create_and_call_contract() {
    // pragma solidity ^0.5.0;
    //
    // contract Test {
    //	 function multiply(uint a, uint b) public pure returns(uint) {
    // 	 	return a * b;
    // 	 }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063165c4a1614602d575b600080fd5b606060048036036040811015604157600080fd5b8101908080359060200190929190803590602001909291905050506076565b6040518082815260200191505060405180910390f35b600081830290509291505056fea265627a7a723158201f3db7301354b88b310868daf4395a6ab6cd42d16b1d8e68cdf4fdd9d34fffbf64736f6c63430005110032").unwrap();

    new_test_ext().execute_with(|| {
		// deploy contract
		let caller = alice();
		let result = Runner::<Test>::create(
			caller.clone(),
			contract,
			0,
			1000000,
			1000000,
			<Test as Config>::config(),
		).unwrap();
		assert_eq!(result.exit_reason, ExitReason::Succeed(ExitSucceed::Returned));

		let contract_address = result.address;

		#[cfg(not(feature = "with-ethereum-compatibility"))]
		deploy_free(contract_address);

		assert_eq!(contract_address, H160::from_str("5f8bd49cd9f0cb2bd5bb9d4320dfe9b61023249d").unwrap());

		assert_eq!(Pallet::<Test>::account_basic(&caller).nonce, U256::from_str("02").unwrap());

		// multiply(2, 3)
		let multiply = from_hex("0x165c4a1600000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003").unwrap();

		// call method `multiply`
		let result = Runner::<Test>::call(
			alice(),
			alice(),
			contract_address,
			multiply,
			0,
			1000000,
			1000000,
			<Test as Config>::config(),
		).unwrap();
		assert_eq!(
			U256::from(6),
			U256::from_big_endian(result.output.as_slice())
		);

		assert_eq!(Pallet::<Test>::account_basic(&caller).nonce, U256::from_str("03").unwrap());

		assert_eq!(Pallet::<Test>::account_basic(&contract_address).nonce, U256::from_str("01").unwrap());
	});
}

#[test]
fn create_reverts_with_message() {
    // pragma solidity ^0.5.0;
    //
    // contract Foo {
    //     constructor() public {
    // 		require(false, "error message");
    // 	}
    // }
    let contract = from_hex("0x6080604052348015600f57600080fd5b5060006083576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252600d8152602001807f6572726f72206d6573736167650000000000000000000000000000000000000081525060200191505060405180910390fd5b603e8060906000396000f3fe6080604052600080fdfea265627a7a723158204741083d83bf4e3ee8099dd0b3471c81061237c2e8eccfcb513dfa4c04634b5b64736f6c63430005110032").expect("invalid hex");
    new_test_ext().execute_with(|| {
        let result = Runner::<Test>::create(
            alice(),
            contract,
            0,
            12_000_000,
            12_000_000,
            <Test as Config>::config(),
        )
        .unwrap();
        assert_eq!(result.exit_reason, ExitReason::Revert(ExitRevert::Reverted));
        assert!(String::from_utf8_lossy(&result.output).contains("error message"));
    });
}

#[test]
fn create_network_contract_works() {
    // pragma solidity ^0.5.0;
    //
    // contract Test {
    //	 function multiply(uint a, uint b) public pure returns(uint) {
    // 	 	return a * b;
    // 	 }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063165c4a1614602d575b600080fd5b606060048036036040811015604157600080fd5b8101908080359060200190929190803590602001909291905050506076565b6040518082815260200191505060405180910390f35b600081830290509291505056fea265627a7a723158201f3db7301354b88b310868daf4395a6ab6cd42d16b1d8e68cdf4fdd9d34fffbf64736f6c63430005110032").unwrap();

    new_test_ext().execute_with(|| {
        // deploy contract
        assert_ok!(EVM::create_network_contract(
            Origin::signed(NetworkContractAccount::get()),
            contract,
            0,
            1000000,
            1000000,
        ));

        assert_eq!(
            Pallet::<Test>::account_basic(&NetworkContractSource::get()).nonce,
            U256::from_str("02").unwrap()
        );

        let created_event = Event::EVM(crate::Event::Created(
            H160::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            H160::from_str("0x2000000000000000000000000000000000000001").unwrap(),
            (Zero::zero(), Zero::zero()),
        ));
        assert!(System::events()
            .iter()
            .any(|record| record.event == created_event));

        assert_eq!(
            EVM::network_contract_index(),
            primitives::NETWORK_CONTRACT_START + 1
        );
    });
}

#[test]
fn create_network_contract_fails_if_non_network_contract_origin() {
    // pragma solidity ^0.5.0;
    //
    // contract Test {
    //	 function multiply(uint a, uint b) public pure returns(uint) {
    // 	 	return a * b;
    // 	 }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063165c4a1614602d575b600080fd5b606060048036036040811015604157600080fd5b8101908080359060200190929190803590602001909291905050506076565b6040518082815260200191505060405180910390f35b600081830290509291505056fea265627a7a723158201f3db7301354b88b310868daf4395a6ab6cd42d16b1d8e68cdf4fdd9d34fffbf64736f6c63430005110032").unwrap();

    new_test_ext().execute_with(|| {
        assert_noop!(
            EVM::create_network_contract(
                Origin::signed(AccountId32::from([1u8; 32])),
                contract,
                0,
                1000000,
                1000000
            ),
            BadOrigin
        );
    });
}

#[test]
fn create_extrinisic_should_deposit_create_event() {
    // pragma solidity ^0.5.0;
    //
    // contract Test {
    //	 function multiply(uint a, uint b) public pure returns(uint) {
    // 	 	return a * b;
    // 	 }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063165c4a1614602d575b600080fd5b606060048036036040811015604157600080fd5b8101908080359060200190929190803590602001909291905050506076565b6040518082815260200191505060405180910390f35b600081830290509291505056fea265627a7a723158201f3db7301354b88b310868daf4395a6ab6cd42d16b1d8e68cdf4fdd9d34fffbf64736f6c63430005110032").unwrap();

    new_test_ext().execute_with(|| {
        let alice_account_id = <Test as Config>::AddressMapping::get_account_id(&alice());
        assert_ok!(EVM::create(
            Origin::signed(alice_account_id),
            contract,
            0,
            1000000,
            1000000
        ));
        let event = crate::Event::Created(
            alice(),
            H160::from_str("0x5f8bd49cd9f0cb2bd5bb9d4320dfe9b61023249d").unwrap(),
            (61183, 284),
        );
        assert!(<frame_system::Pallet<Test>>::events()
            .iter()
            .any(|x| x.event == mock::RuntimeEvent::EVM(event.clone())));
    });
}

#[test]
fn create2_extrinisic_should_deposit_create_event() {
    // pragma solidity ^0.5.0;
    //
    // contract Test {
    //	 function multiply(uint a, uint b) public pure returns(uint) {
    // 	 	return a * b;
    // 	 }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063165c4a1614602d575b600080fd5b606060048036036040811015604157600080fd5b8101908080359060200190929190803590602001909291905050506076565b6040518082815260200191505060405180910390f35b600081830290509291505056fea265627a7a723158201f3db7301354b88b310868daf4395a6ab6cd42d16b1d8e68cdf4fdd9d34fffbf64736f6c63430005110032").unwrap();

    new_test_ext().execute_with(|| {
        let alice_account_id = <Test as Config>::AddressMapping::get_account_id(&alice());
        assert_ok!(EVM::create2(
            Origin::signed(alice_account_id),
            contract,
            H256::from_str("0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
                .unwrap(),
            0,
            1000000,
            1000000
        ));
        let event = crate::Event::Created(
            alice(),
            H160::from_str("0x182f69c8cd38252a33d1a38c48c6fcf8a1742086").unwrap(),
            (61183, 284),
        );
        assert!(<frame_system::Pallet<Test>>::events()
            .iter()
            .any(|x| x.event == mock::RuntimeEvent::EVM(event.clone())));
    });
}

#[cfg(feature = "with-ethereum-compatibility")]
#[test]
fn call_extrinsic_should_deposit_create_event() {
    // // SPDX-License-Identifier: MIT
    // pragma solidity ^0.8.4;
    //
    // contract Factory {
    //     TargetContract[] newContracts;
    //     function createContract (uint num) public {
    //         for(uint i = 0; i < num; i++) {
    //             TargetContract newContract = new TargetContract();
    //             newContracts.push(newContract);
    //         }
    //     }
    // }
    //
    // contract TargetContract {
    //     function hello() pure public returns (string memory){
    //         return "hello";
    //     }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b506103be806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c80639db8d7d514610030575b600080fd5b61004a60048036038101906100459190610121565b61004c565b005b60005b818110156100fb576000604051610065906100ff565b604051809103906000f080158015610081573d6000803e3d6000fd5b5090506000819080600181540180825580915050600190039060005260206000200160009091909190916101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055505080806100f390610158565b91505061004f565b5050565b61019c806101ed83390190565b60008135905061011b816101d5565b92915050565b600060208284031215610137576101366101d0565b5b60006101458482850161010c565b91505092915050565b6000819050919050565b60006101638261014e565b91507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff821415610196576101956101a1565b5b600182019050919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b600080fd5b6101de8161014e565b81146101e957600080fd5b5056fe608060405234801561001057600080fd5b5061017c806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c806319ff1d2114610030575b600080fd5b61003861004e565b60405161004591906100c4565b60405180910390f35b60606040518060400160405280600581526020017f68656c6c6f000000000000000000000000000000000000000000000000000000815250905090565b6000610096826100e6565b6100a081856100f1565b93506100b0818560208601610102565b6100b981610135565b840191505092915050565b600060208201905081810360008301526100de818461008b565b905092915050565b600081519050919050565b600082825260208201905092915050565b60005b83811015610120578082015181840152602081019050610105565b8381111561012f576000848401525b50505050565b6000601f19601f830116905091905056fea264697066735822122071d4074b81cec433daf9acecb8ca8d5de5fce5d71fe1d2ec5ada11d1153ec63464736f6c63430008070033a26469706673582212202272533c150b805367c9e3ac96ce5cd243e0ca32e2178360fb1867c7bdb4de0864736f6c63430008070033").unwrap();

    new_test_ext().execute_with(|| {
        assert_ok!(EVM::create(
            Origin::signed(<Test as Config>::AddressMapping::get_account_id(&alice())),
            contract,
            0,
            1000000,
            1000000
        ));
        let address = H160::from_str("0x5f8bd49cd9f0cb2bd5bb9d4320dfe9b61023249d").unwrap();
        assert_ok!(EVM::call(
            Origin::signed(<Test as Config>::AddressMapping::get_account_id(&alice())),
            address,
            from_hex("0x9db8d7d50000000000000000000000000000000000000000000000000000000000000002")
                .unwrap(), // Factory.createContract(2)
            0,
            1000000,
            1000000,
        ));

        // We expect two Created events as we have specified argument num = 2 in call
        let event = mock::Event::from(crate::Event::Created(
            alice(),
            H160::from_str("0x7b8f8ca099f6e33cf1817cf67d0556429cfc54e4").unwrap(),
            (841916, 480),
        ));
        assert!(<frame_system::Pallet<Test>>::events()
            .iter()
            .any(|x| x.event == event));

        let event = mock::Event::from(crate::Event::Created(
            alice(),
            H160::from_str("0x39b26a36a8a175ce7d498b5ef187d1ab2f381bbd").unwrap(),
            (694031, 480),
        ));
        assert!(<frame_system::Pallet<Test>>::events()
            .iter()
            .any(|x| x.event == event));
    })
}

#[cfg(feature = "with-ethereum-compatibility")]
#[test]
fn call_extrinsic_should_not_deposit_create_event_on_revert() {
    // // SPDX-License-Identifier: MIT
    // pragma solidity ^0.8.4;
    //
    // contract Factory {
    //     TargetContract[] newContracts;
    //     function createContract (uint num) public {
    //         for(uint i = 0; i < num; i++) {
    //             TargetContract newContract = new TargetContract();
    //             newContracts.push(newContract);
    //         };
    //         require(false);
    //     }
    // }
    //
    // contract TargetContract {
    //     function hello() pure public returns (string memory){
    //         return "hello";
    //     }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b506103c9806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c80639db8d7d514610030575b600080fd5b61004a6004803603810190610045919061012c565b61004c565b005b60005b818110156100fb5760006040516100659061010a565b604051809103906000f080158015610081573d6000803e3d6000fd5b5090506000819080600181540180825580915050600190039060005260206000200160009091909190916101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055505080806100f390610163565b91505061004f565b50600061010757600080fd5b50565b61019c806101f883390190565b600081359050610126816101e0565b92915050565b600060208284031215610142576101416101db565b5b600061015084828501610117565b91505092915050565b6000819050919050565b600061016e82610159565b91507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff8214156101a1576101a06101ac565b5b600182019050919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b600080fd5b6101e981610159565b81146101f457600080fd5b5056fe608060405234801561001057600080fd5b5061017c806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c806319ff1d2114610030575b600080fd5b61003861004e565b60405161004591906100c4565b60405180910390f35b60606040518060400160405280600581526020017f68656c6c6f000000000000000000000000000000000000000000000000000000815250905090565b6000610096826100e6565b6100a081856100f1565b93506100b0818560208601610102565b6100b981610135565b840191505092915050565b600060208201905081810360008301526100de818461008b565b905092915050565b600081519050919050565b600082825260208201905092915050565b60005b83811015610120578082015181840152602081019050610105565b8381111561012f576000848401525b50505050565b6000601f19601f830116905091905056fea2646970667358221220e856fea14d533e3bcdc2fb2d2504e31e0ee5b7b6c9eeb4ec87dae91604ab56d864736f6c63430008070033a264697066735822122084fb88be56a4a1a65de42f5e4d933f216656a6d1e4d400c702f0a911a50955e664736f6c63430008070033").unwrap();

    new_test_ext().execute_with(|| {
        assert_ok!(EVM::create(
            Origin::signed(<Test as Config>::AddressMapping::get_account_id(&alice())),
            contract,
            0,
            1000000,
            1000000
        ));
        let address = H160::from_str("0x5f8bd49cd9f0cb2bd5bb9d4320dfe9b61023249d").unwrap();
        assert_ok!(EVM::call(
            Origin::signed(<Test as Config>::AddressMapping::get_account_id(&alice())),
            address,
            from_hex("0x9db8d7d50000000000000000000000000000000000000000000000000000000000000002")
                .unwrap(), // Factory.createContract(2)
            0,
            1000000,
            1000000,
        ));

        // We confirm that Created events have not been emitted
        for event in <frame_system::Pallet<Test>>::events().iter() {
            if let Event::EVM(crate::Event::Created(_, address, ..)) = event.event {
                assert!(
                    address
                        != H160::from_str("0x7b8f8ca099f6e33cf1817cf67d0556429cfc54e4").unwrap()
                );
                assert!(
                    address
                        != H160::from_str("0x39b26a36a8a175ce7d498b5ef187d1ab2f381bbd").unwrap()
                );
            }
        }
    })
}

#[cfg(not(feature = "with-ethereum-compatibility"))]
#[test]
fn should_deploy_free() {
    // pragma solidity ^0.5.0;
    //
    // contract Test {
    //	 function multiply(uint a, uint b) public pure returns(uint) {
    // 	 	return a * b;
    // 	 }
    // }
    let contract = from_hex("0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063165c4a1614602d575b600080fd5b606060048036036040811015604157600080fd5b8101908080359060200190929190803590602001909291905050506076565b6040518082815260200191505060405180910390f35b600081830290509291505056fea265627a7a723158201f3db7301354b88b310868daf4395a6ab6cd42d16b1d8e68cdf4fdd9d34fffbf64736f6c63430005110032").unwrap();

    new_test_ext().execute_with(|| {
		// contract not created yet
		assert_noop!(EVM::deploy_free(Origin::signed(CouncilAccount::get()), H160::default()), Error::<Test>::ContractNotFound);

		// create contract
		let result = Runner::<Test>::create(alice(), contract, 0, 21_000_000, 21_000_000, <Test as Config>::config()).unwrap();
		let contract_address = result.address;

		// multiply(2, 3)
		let multiply = from_hex("0x165c4a1600000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003").unwrap();

		// call method `multiply` will fail, not deployed yet
		assert_noop!(Runner::<Test>::call(
			alice(),
			alice(),
			contract_address,
			multiply.clone(),
			0,
			1000000,
			1000000,
			<Test as Config>::config(),
		), Error::<Test>::NoPermission);

		assert_ok!(EVM::deploy_free(Origin::signed(CouncilAccount::get()), contract_address));

		// multiply(2, 3)
		let multiply = from_hex("0x165c4a1600000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003").unwrap();

		// call method `multiply`
		assert_ok!(Runner::<Test>::call(
			alice(),
			alice(),
			contract_address,
			multiply.clone(),
			0,
			1000000,
			1000000,
			<Test as Config>::config(),
		));

		// contract already deployed
		assert_noop!(EVM::deploy_free(Origin::signed(CouncilAccount::get()), contract_address), Error::<Test>::ContractAlreadyDeployed);
	});
}

#[test]
fn should_enable_contract_development() {
    new_test_ext().execute_with(|| {
        let alice_account_id = <Test as Config>::AddressMapping::get_account_id(&alice());
        assert_ok!(EVM::enable_contract_development(Origin::signed(
            alice_account_id
        )));
        assert_eq!(
            Accounts::<Test>::get(alice()).unwrap().developer_deposit,
            Some(DeveloperDeposit::get())
        );
        assert_eq!(balance(alice()), INITIAL_BALANCE - DeveloperDeposit::get());
    });
}

#[test]
fn should_disable_contract_development() {
    new_test_ext().execute_with(|| {
        let alice_account_id = <Test as Config>::AddressMapping::get_account_id(&alice());

        // contract development is not enabled yet
        assert_noop!(
            EVM::disable_contract_development(Origin::signed(alice_account_id.clone())),
            Error::<Test>::ContractDevelopmentNotEnabled
        );
        assert_eq!(balance(alice()), INITIAL_BALANCE);

        // enable contract development
        assert_ok!(EVM::enable_contract_development(Origin::signed(
            alice_account_id.clone()
        )));
        assert_eq!(
            Accounts::<Test>::get(alice()).unwrap().developer_deposit,
            Some(DeveloperDeposit::get())
        );

        // deposit reserved
        assert_eq!(balance(alice()), INITIAL_BALANCE - DeveloperDeposit::get());

        // disable contract development
        assert_ok!(EVM::disable_contract_development(Origin::signed(
            alice_account_id.clone()
        )));
        // deposit unreserved
        assert_eq!(balance(alice()), INITIAL_BALANCE);

        // contract development already disabled
        assert_noop!(
            EVM::disable_contract_development(Origin::signed(alice_account_id)),
            Error::<Test>::ContractDevelopmentNotEnabled
        );
    });
}

#[cfg(feature = "with-ethereum-compatibility")]
#[test]
fn should_selfdestruct_via_evm_call() {
    // pragma solidity ^0.8.4;
    //
    // contract Test {
    // 	address payable private owner;
    //
    // 	constructor() {
    // 		owner = payable(msg.sender);
    // 	}
    //
    // 	function close () public {
    // 		selfdestruct(owner);
    // 	}
    // }
    let contract = from_hex("0x6080604052348015600f57600080fd5b50336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060a48061005e6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c806343d726d614602d575b600080fd5b60336035565b005b60008054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16fffea26469706673582212207dacfac5a65fdfa63f19717f68c02f0b95d1548af0cdb1fb3ffe5c7384ecc29b64736f6c63430008070033").unwrap();

    new_test_ext().execute_with(|| {
        let alice_account_id = <Test as Config>::AddressMapping::get_account_id(&alice());

        // create contract
        let result = Runner::<Test>::create(
            alice(),
            contract.clone(),
            0,
            21_000_000,
            21_000_000,
            <Test as Config>::config(),
        )
        .unwrap();
        let contract_address = result.address;
        let code_hash = Accounts::<Test>::get(contract_address)
            .unwrap()
            .contract_info
            .unwrap()
            .code_hash;
        assert_eq!(result.used_storage, 328);
        let alice_balance = INITIAL_BALANCE - 328 * <Test as Config>::StorageDepositPerByte::get();

        assert_eq!(balance(alice()), alice_balance);

        assert_ok!(EVM::call(
            Origin::signed(<Test as Config>::AddressMapping::get_account_id(&alice())),
            contract_address,
            from_hex("0x43d726d6").unwrap(), // Test.close()
            0,
            1000000,
            1000000,
        ));
        let event = Event::EVM(crate::Event::ContractSelfdestructed(
            alice(),
            contract_address,
        ));
        assert!(System::events().iter().any(|record| record.event == event));

        // assert storage cleanup successful
        assert_eq!(Accounts::<Test>::get(contract_address), None);
        assert_eq!(
            AccountStorages::<Test>::iter_key_prefix(contract_address).count(),
            0
        );
        assert_eq!(CodeInfos::<Test>::get(code_hash), None);
        assert_eq!(Codes::<Test>::get(code_hash), Vec::<u8>::new());

        // assert refund of resereved balance, minus storage deposit and storage costs
        assert_eq!(balance(alice()), alice_balance);
    });
}
