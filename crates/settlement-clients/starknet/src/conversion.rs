use starknet::core::types::FieldElement;

pub fn u8_to_vec(i: &[u8]) -> Vec<FieldElement> {
    i.iter().map(|&byte| FieldElement::from(byte)).collect()
}
