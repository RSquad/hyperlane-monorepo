pub mod conversion {
    use anyhow::Error;
    use base64::{engine::general_purpose, Engine};
    use num_bigint::BigUint;
    use tonlib_core::{TonAddress, TonHash};

    use hyperlane_core::{ChainCommunicationError, H256, H512, U256};

    use crate::{
        error::HyperlaneTonError,
        run_get_method::{StackItem, StackValue},
    };

    pub fn base64_to_h512(hash: &str) -> Result<H512, Error> {
        let mut padded = [0u8; 64];
        general_purpose::STANDARD
            .decode_slice(hash, &mut padded)
            .map_err(|e| Error::msg(format!("Failed to decode base64 hash: {}", e)))?;

        Ok(H512::from_slice(&padded))
    }
    pub fn base64_to_h256(hash: &str) -> Result<H256, Error> {
        let decoded_bytes = general_purpose::STANDARD
            .decode(hash)
            .map_err(|e| Error::msg(format!("Failed to decode base64: {}", e)))?;

        if decoded_bytes.len() != 32 {
            return Err(Error::msg(format!(
                "Decoded bytes length is {}. Expected 32 bytes.",
                decoded_bytes.len()
            )));
        }

        Ok(H256::from_slice(&decoded_bytes))
    }

    pub fn ton_address_to_h256(address: &TonAddress) -> H256 {
        H256::from_slice(address.hash_part.as_slice())
    }

    pub fn u256_to_biguint(value: U256) -> BigUint {
        let mut bytes = [0u8; 32]; // 256 bit = 32 byte
        value.to_little_endian(&mut bytes);
        BigUint::from_bytes_le(&bytes)
    }
    pub fn h256_to_ton_address(h256: &H256, workchain: i32) -> TonAddress {
        TonAddress::new(workchain, &TonHash::from(&h256.0))
    }
    pub fn extract_boc_from_stack_item(
        stack_item: &StackItem,
    ) -> Result<&String, ChainCommunicationError> {
        match &stack_item.value {
            StackValue::String(boc) => Ok(boc),
            _ => Err(ChainCommunicationError::from(
                HyperlaneTonError::ParsingError(format!(
                    "Failed to get boc: unexpected data type: {:?}",
                    stack_item.value
                )),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::conversion::*;
    use crate::run_get_method::{StackItem, StackValue};
    use hyperlane_core::{H256, H512, U256};
    use num_bigint::BigUint;
    use num_traits::Zero;
    use tonlib_core::TonAddress;
    #[test]
    fn test_base64_to_h512_valid() {
        let hash_str = "emUQnddCZvrUNaMmy0eYGzRtHAVsdniV0x7EBpK6ON4=";
        let expected = H512::from_slice(&[
            0x7a, 0x65, 0x10, 0x9d, 0xd7, 0x42, 0x66, 0xfa, 0xd4, 0x35, 0xa3, 0x26, 0xcb, 0x47,
            0x98, 0x1b, 0x34, 0x6d, 0x1c, 0x05, 0x6c, 0x76, 0x78, 0x95, 0xd3, 0x1e, 0xc4, 0x06,
            0x92, 0xba, 0x38, 0xde, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        let result = base64_to_h512(hash_str).expect("Conversion failed");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_base64_to_h512_invalid() {
        let invalid_hash_str = "invalid_base64_string";

        let result = base64_to_h512(invalid_hash_str);
        assert!(result.is_err(), "Expected an error for invalid input");
    }

    #[test]
    fn test_base64_to_h256_invalid_length() {
        let base64_hash = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let result = base64_to_h256(base64_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_u256_to_biguint_zero() {
        // Create a U256 value of 0
        let u256_value = U256::zero();

        // Convert to BigUint
        let biguint_value = u256_to_biguint(u256_value);

        // Verify correctness
        assert_eq!(biguint_value, BigUint::zero());
    }
    #[test]
    fn test_u256_to_biguint_conversion() {
        // Create a U256 value
        let u256_value = U256::from_dec_str("1234567890123456789012345678901234567890").unwrap();

        // Convert to BigUint
        let biguint_value = u256_to_biguint(u256_value);

        // Verify correctness
        let expected_value =
            BigUint::parse_bytes(b"1234567890123456789012345678901234567890", 10).unwrap();
        assert_eq!(biguint_value, expected_value);
    }
    #[test]
    fn test_ton_address_to_h256() {
        let address =
            TonAddress::from_base64_url("UQCvsB60DElBwHpHOj26K9NfxGJgzes_5pzwV48QGxHar2F3")
                .unwrap();
        let result = ton_address_to_h256(&address);
        let expected = H256::from_slice(&[
            0xaf, 0xb0, 0x1e, 0xb4, 0x0c, 0x49, 0x41, 0xc0, 0x7a, 0x47, 0x3a, 0x3d, 0xba, 0x2b,
            0xd3, 0x5f, 0xc4, 0x62, 0x60, 0xcd, 0xeb, 0x3f, 0xe6, 0x9c, 0xf0, 0x57, 0x8f, 0x10,
            0x1b, 0x11, 0xda, 0xaf,
        ]);

        assert_eq!(result, expected);
    }
    #[test]
    fn test_h256_to_ton_address() {
        let h256 = H256::from_slice(&[0x12; 32]);
        let workchain = 0;

        let address = h256_to_ton_address(&h256, workchain);

        assert_eq!(address.workchain, workchain);
        assert_eq!(address.hash_part.as_slice(), h256.as_bytes());
    }

    #[test]
    fn test_extract_boc_from_stack_item() {
        let stack_item = StackItem {
            r#type: "cell".to_string(),
            value: StackValue::String("te6cckEBAgEATQABQ6AGvFm965B0z/96EKlW2xGIv+qjfKDHQWY2NlXkdJEINtABAEz3CxqLkN5V+jk24kdOlIIhNfGZYWH0y0ato9U/6pMBogAAAAAAAZnmUE8=".to_string()),
        };

        let boc = extract_boc_from_stack_item(&stack_item).unwrap();

        assert_eq!(boc, "te6cckEBAgEATQABQ6AGvFm965B0z/96EKlW2xGIv+qjfKDHQWY2NlXkdJEINtABAEz3CxqLkN5V+jk24kdOlIIhNfGZYWH0y0ato9U/6pMBogAAAAAAAZnmUE8=");
    }
    #[test]
    fn test_extract_boc_from_stack_item_invalid_type() {
        let stack_item = StackItem {
            r#type: "list".to_string(),
            value: StackValue::List(vec![]),
        };

        let result = extract_boc_from_stack_item(&stack_item);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Data parsing error: Failed to get boc: unexpected data type: List([])"
        );
    }
}
