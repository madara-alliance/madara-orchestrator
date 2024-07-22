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
    fn test_try_non_zero_u128_from_u128_valid() {
        let result = try_non_zero_u128_from_u128(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get(), 42);
    }

    #[test]
    fn test_try_non_zero_u128_from_u128_max() {
        let result = try_non_zero_u128_from_u128(u128::MAX);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get(), u128::MAX);
    }

    #[test]
    fn test_try_non_zero_u128_from_u128_one() {
        let result = try_non_zero_u128_from_u128(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get(), 1);
    }

    #[test]
    fn test_try_non_zero_u128_from_u128_zero() {
        let result = try_non_zero_u128_from_u128(0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Could not convert 0 from u128 to NonZeroU128.");
    }
}
