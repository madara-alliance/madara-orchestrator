use starknet::core::types::Felt;

pub(crate) fn slice_slice_u8_to_vec_field(slices: &[[u8; 32]]) -> Vec<Felt> {
    slices.iter().map(slice_u8_to_field).collect()
}

pub(crate) fn slice_u8_to_field(slice: &[u8; 32]) -> Felt {
    Felt::from_bytes_be_slice(slice)
}
