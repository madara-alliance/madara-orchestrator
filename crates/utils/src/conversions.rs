use std::num::NonZeroU128;

use color_eyre::eyre::{eyre, Result};

pub fn try_non_zero_u128_from_u128(v: u128) -> Result<NonZeroU128> {
    let non_zero = NonZeroU128::new(v).ok_or_else(|| eyre!("Could not convert {v} from u128 to NonZeroU128."))?;
    Ok(non_zero)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_non_zero_u128_from_u128() {
        // Test valid conversions
        assert_eq!(try_non_zero_u128_from_u128(42).unwrap().get(), 42);
        assert_eq!(try_non_zero_u128_from_u128(u128::MAX).unwrap().get(), u128::MAX);

        // Test zero (invalid conversion)
        let err = try_non_zero_u128_from_u128(0).unwrap_err();
        assert_eq!(err.to_string(), "Could not convert 0 from u128 to NonZeroU128.");
    }
}
