use starknet::core::types::FieldElement;

/// Converts a `&[Vec<u8>]` to `Vec<U256>`. Each inner slice is expected to
/// be exactly 32 bytes long.
/// Pads with zeros if any inner slice is shorter than 32 bytes.
pub(crate) fn slice_slice_u8_to_vec_field(slices: &[Vec<u8>]) -> Vec<FieldElement> {
    slices.iter().map(|slice| slice_u8_to_field(slice)).collect()
}

/// Converts a `&[u8]` to `U256`. Expects the input slice to be exactly 32 bytes long.
/// Pads with zeros if the input is shorter than 32 bytes.
pub(crate) fn slice_u8_to_field(slice: &[u8]) -> FieldElement {
    let mut fixed_bytes = [0u8; 32];
    let len = slice.len();
    if len <= 32 {
        fixed_bytes[..len].copy_from_slice(slice);
    } else {
        fixed_bytes.copy_from_slice(&slice[..32]);
    }
    // TODO(akhercha): handle conversion error
    FieldElement::from_bytes_be(&fixed_bytes).expect("mmh")
}
