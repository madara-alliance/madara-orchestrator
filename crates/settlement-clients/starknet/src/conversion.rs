use color_eyre::Result;
use starknet::core::types::Felt;

pub(crate) fn slice_slice_u8_to_vec_field(slices: &[[u8; 32]]) -> Vec<Felt> {
    slices.iter().map(slice_u8_to_field).collect()
}

pub(crate) fn slice_u8_to_field(slice: &[u8; 32]) -> Felt {
    Felt::from_bytes_be_slice(slice)
}

pub(crate) fn u64_from_felt(number: Felt) -> Result<u64> {
    let bytes = number.to_bytes_be();

    for x in &bytes[0..24] {
        assert!(*x == 0, "byte should be zero, cannot convert to Felt");
    }
    Ok(u64::from_be_bytes(bytes[24..32].try_into().unwrap()))
}

#[test]
fn test_u64_from_from_felt_ok() {
    let number = 10.into();
    let converted = u64_from_felt(number);
    assert!(converted.unwrap() == 10u64, "Should be able to convert");
}

#[test]
#[should_panic(expected = "byte should be zero, cannot convert to Felt")]
fn test_u64_from_from_felt_panic() {
    let number = Felt::MAX;
    u64_from_felt(number).unwrap();
}
