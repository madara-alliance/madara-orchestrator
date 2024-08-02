use alloy::primitives::U256;
/// Converts a `&[Vec<u8>]` to `Vec<U256>`. Each inner slice is expected to be exactly 32 bytes long.
/// Pads with zeros if any inner slice is shorter than 32 bytes.
pub(crate) fn slice_slice_u8_to_vec_u256(slices: &[[u8; 32]]) -> Vec<U256> {
    slices.iter().map(|slice| slice_u8_to_u256(slice)).collect()
}

/// Converts a `&[u8]` to `U256`.
pub(crate) fn slice_u8_to_u256(slice: &[u8]) -> U256 {
    U256::try_from_be_slice(slice).expect("could not convert u8 slice to U256")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::typical(&[
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF
    ], U256::from_str_radix("00112233445566778899AABBCCDDEEFF00112233445566778899AABBCCDDEEFF", 16).unwrap())]
    #[case::minimum(&[0; 32], U256::ZERO)]
    #[case::maximum(&[0xFF; 32], U256::MAX)]
    #[case::short(&[0xFF; 16], U256::from_be_slice(&[0xFF; 16]))]
    #[case::empty(&[], U256::ZERO)]
    fn slice_u8_to_u256_works(#[case] slice: &[u8], #[case] expected: U256) {
        assert_eq!(slice_u8_to_u256(slice), expected);
    }

    #[rstest]
    #[should_panic]
    #[case::over(&[0xFF; 33])]
    fn slice_u8_to_u256_panics(#[case] slice: &[u8]) {
        slice_u8_to_u256(slice);
    }

    #[rstest]
    #[case::empty(&[], vec![])]
    #[case::single(
        &[[1; 32]],
        vec![U256::from_be_slice(&[1; 32])]
    )]
    #[case::multiple(
        &[
            [1; 32],
            [2; 32],
            [3; 32],
        ],
        vec![
            U256::from_be_slice(&[1; 32]),
            U256::from_be_slice(&[2; 32]),
            U256::from_be_slice(&[3; 32]),
        ]
    )]
    #[case::mixed(
        &[
            [0xFF; 32],
            [0x00; 32],
            [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ],
        vec![
            U256::MAX,
            U256::ZERO,
            U256::from_be_slice(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ]
    )]
    fn slice_slice_u8_to_vec_u256_works(#[case] slices: &[[u8; 32]], #[case] expected: Vec<U256>) {
        let response = slice_slice_u8_to_vec_u256(slices);
        assert_eq!(response, expected);
    }
}
