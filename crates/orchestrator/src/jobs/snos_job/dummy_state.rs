//! A Dummy state that does nothing.
//! It just implements the State and StateReader traits provided by Blockifier.
//!
//! This module needs to be deleted as soon as we can import the structure
//! [BlockifierStateAdapter] from Madara code (Currently, we have version
//! conflicts between snos <=> deoxys <=> cairo-vm)
//! OR
//! if it's not needed at all following the Snos code update. This update
//! will make the run of the OS easier to integrate with Madara, it may
//! not be needed to pass a State object.

use std::collections::HashSet;

use blockifier::execution::contract_class::{ContractClass, ContractClassV0};
use blockifier::state::cached_state::CommitmentStateDiff;
use blockifier::state::state_api::{State, StateReader, StateResult};
use indexmap::IndexMap;
use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;

pub struct DummyState;

impl StateReader for DummyState {
    fn get_storage_at(&mut self, _contract_address: ContractAddress, _key: StorageKey) -> StateResult<StarkFelt> {
        Ok(StarkFelt::ZERO)
    }

    fn get_nonce_at(&mut self, _contract_address: ContractAddress) -> StateResult<Nonce> {
        Ok(Nonce::default())
    }

    fn get_class_hash_at(&mut self, _contract_address: ContractAddress) -> StateResult<ClassHash> {
        Ok(ClassHash::default())
    }

    fn get_compiled_contract_class(&mut self, _class_hash: ClassHash) -> StateResult<ContractClass> {
        Ok(ContractClass::V0(ContractClassV0::default()))
    }

    fn get_compiled_class_hash(&mut self, _class_hash: ClassHash) -> StateResult<CompiledClassHash> {
        Ok(CompiledClassHash::default())
    }
}

impl State for DummyState {
    fn set_storage_at(
        &mut self,
        _contract_address: ContractAddress,
        _key: StorageKey,
        _value: StarkFelt,
    ) -> StateResult<()> {
        Ok(())
    }

    fn increment_nonce(&mut self, _contract_address: ContractAddress) -> StateResult<()> {
        Ok(())
    }

    fn set_class_hash_at(&mut self, _contract_address: ContractAddress, _class_hash: ClassHash) -> StateResult<()> {
        Ok(())
    }

    fn set_contract_class(&mut self, _class_hash: ClassHash, _contract_class: ContractClass) -> StateResult<()> {
        Ok(())
    }

    fn set_compiled_class_hash(
        &mut self,
        _class_hash: ClassHash,
        _compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        Ok(())
    }

    fn to_state_diff(&mut self) -> CommitmentStateDiff {
        CommitmentStateDiff {
            address_to_class_hash: IndexMap::default(),
            address_to_nonce: IndexMap::default(),
            storage_updates: IndexMap::default(),
            class_hash_to_compiled_class_hash: IndexMap::default(),
        }
    }

    fn add_visited_pcs(&mut self, _class_hash: ClassHash, _pcs: &HashSet<usize>) {}
}
