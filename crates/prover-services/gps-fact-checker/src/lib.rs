pub mod error;
pub mod fact_info;
pub mod fact_node;
pub mod fact_topology;

use alloy::primitives::{Address, B256};
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::sol;
use alloy::transports::http::{Client, Http};
use url::Url;

use self::error::FactCheckerError;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    FactRegistry,
    "tests/artifacts/FactRegistry2.json"
);

pub struct FactChecker {
    fact_registry: FactRegistry::FactRegistryInstance<TransportT, ProviderT>,
}

type TransportT = Http<Client>;
type ProviderT = RootProvider<TransportT>;

impl FactChecker {
    pub fn new(rpc_node_url: Url, verifier_address: Address) -> Self {
        let provider = ProviderBuilder::new().on_http(rpc_node_url);
        let fact_registry = FactRegistry::new(verifier_address, provider);
        Self { fact_registry }
    }

    pub async fn is_valid(&self, fact: &B256) -> Result<bool, FactCheckerError> {
        let FactRegistry::isValidReturn { _0 } =
            self.fact_registry.isValid(*fact).call().await.map_err(FactCheckerError::FactRegistry)?;
        Ok(_0)
    }
}

#[cfg(test)]
mod tests {
    use crate::FactChecker;
    use alloy::primitives::{Address, B256};
    use rstest::rstest;
    use std::str::FromStr;
    use url::Url;
    use utils::env_utils::get_env_var_or_panic;

    #[rstest]
    // Picked a valid fact registered on Eth.
    // You can check on etherscan by calling `isValid` function on GpsStatementVerifier.sol
    // Contract Link : https://etherscan.io/address/0x9fb7F48dCB26b7bFA4e580b2dEFf637B13751942#readContract#F9
    #[case::valid_fact("0xec8fa9cdfe069ed59b8f17aeecfd95c6abd616379269d2fa16a80955b6e0f068", true)]
    // same fact with last letter changed
    #[case::invalid_fact("0xec8fa9cdfe069ed59b8f17aeecfd95c6abd616379269d2fa16a80955b6e0f067", false)]
    #[tokio::test]
    pub async fn fact_registry_is_valid(#[case] fact: &str, #[case] is_valid: bool) {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

        let fact_checker = FactChecker::new(
            Url::from_str(get_env_var_or_panic("ETHEREUM_MAINNET_RPC_URL").as_str()).unwrap(),
            Address::from_str(get_env_var_or_panic("MEMORY_PAGES_CONTRACT_ADDRESS").as_str()).unwrap(),
        );
        let fact = B256::from_str(fact).unwrap();
        assert_eq!(fact_checker.is_valid(&fact).await.unwrap(), is_valid);
    }
}
