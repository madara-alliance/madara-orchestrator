#[starknet::interface]
pub trait IPiltover<TContractState> {
    fn update_state(
        ref self: TContractState,
        program_output: Span<felt252>,
        onchain_data_hash: felt252,
        onchain_data_size: u256
    );
}

#[starknet::contract]
mod Piltover {
    #[storage]
    struct Storage {
        balance: felt252,
    }

    #[abi(embed_v0)]
    impl IPiltoverImpl of super::IPiltover<ContractState> {
        fn update_state(
            ref self: ContractState,
            program_output: Span<felt252>,
            onchain_data_hash: felt252,
            onchain_data_size: u256
        ) {}
    }
}
