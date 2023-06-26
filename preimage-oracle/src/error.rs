use alloc::string::String;

#[derive(Debug)]
pub enum PreimageOracleError {
    /// The preimage was not found in the oracle.
    PreimageNotFound(String),
    /// The preimage was found in the oracle, but it was not the correct length.
    IncorrectPreimageLength,
    /// The preimage was found in the oracle, but it was not the correct value.
    IncorrectPreimageValue,
    /// Other errors.
    Other(String),
}
