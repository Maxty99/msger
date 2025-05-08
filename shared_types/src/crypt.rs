/// ## Crypt header key
/// Server will send this on succesful connection with the value being encrypted with the set password
/// Client will try to read this value on connection to ensure the correct password was provided
pub const CRYPT_VALIDATION_KEY: &str = "msger_crypt";

/// ## Crypt header value
/// Server will encrypt this with the configured password
/// Client will decrypt with provided password and ensure string matches the expected unencrypted value of "Decrypt me"
pub const CRYPT_VALIDATION_VAL: &str = "Decrypt me";
