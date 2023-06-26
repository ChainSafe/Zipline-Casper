use ethereum_consensus::state_transition::{Context, Result};
use std::fmt;
use test_utils::{load_snappy_ssz, Config};

pub struct AttestationTestCase<S, T> {
    pre: S,
    post: Option<S>,
    operation: T,
    config: Config,
}

impl<S, T> AttestationTestCase<S, T>
where
    S: fmt::Debug + ssz_rs::Deserialize + PartialEq<S>,
    T: ssz_rs::Deserialize,
{
    pub fn from(test_case_path: &str) -> Self {
        let path = test_case_path.to_string() + "/pre.ssz_snappy";
        let pre: S = load_snappy_ssz(&path).unwrap();

        let path = test_case_path.to_string() + "/post.ssz_snappy";
        let post = load_snappy_ssz::<S>(&path);

        let path = test_case_path.to_string() + "/attestation.ssz_snappy";
        let operation: T = load_snappy_ssz(&path).unwrap();

        let config = if test_case_path.contains("minimal") {
            Config::Minimal
        } else {
            Config::Mainnet
        };

        Self {
            pre,
            post,
            operation,
            config,
        }
    }

    pub fn execute<F>(&mut self, f: F)
    where
        F: FnOnce(&mut S, &T, &Context) -> Result<()>,
    {
        let context = match self.config {
            Config::Minimal => Context::for_minimal(),
            Config::Mainnet => Context::for_mainnet(),
        };

        let result = f(&mut self.pre, &self.operation, &context);
        assert!(result.is_ok())
    }
}
