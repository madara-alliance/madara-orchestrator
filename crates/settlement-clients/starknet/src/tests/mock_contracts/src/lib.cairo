#[starknet::interface]
pub trait IPiltover<TContractState> {
    fn update_state(self: @TContractState);
}

#[starknet::contract]
mod Piltover {
    #[storage]
    struct Storage {
        balance: felt252,
    }

    #[abi(embed_v0)]
    impl IPiltoverImpl of super::IPiltover<ContractState> {
        fn update_state(self: @ContractState) {}
    }
}
