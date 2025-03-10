pub mod parsers {
    use std::{collections::HashMap, sync::Arc};

    use base64::{engine::general_purpose, Engine};
    use num_bigint::BigUint;
    use tonlib_core::{
        cell::{
            dict::predefined_readers::{key_reader_uint, val_reader_cell},
            ArcCell, BagOfCells, Cell, TonCellError,
        },
        TonAddress,
    };
    use tracing::{info, warn};

    use hyperlane_core::{
        ChainCommunicationError, ChainResult, TxnInfo, TxnReceiptInfo, H160, H256, H512, U256,
    };

    use crate::transaction::Transaction;
    use crate::{
        error::HyperlaneTonError,
        run_get_method::{StackItem, StackValue},
    };

    /// Parses the root `root_cell` and extracts a dictionary of addresses with their storage locations.
    /// Uses a nested dictionary to store strings in the `BigUint -> Vec<String>` format.
    pub fn parse_address_storage_locations(
        root_cell: &ArcCell,
    ) -> Result<HashMap<BigUint, Vec<String>>, TonCellError> {
        let mut storage_locations: HashMap<BigUint, Vec<String>> = HashMap::new();

        let parsed = root_cell
            .parser()
            .load_dict(256, key_reader_uint, val_reader_cell)?;

        for (key, value_cell) in &parsed {
            let mut storage_list = Vec::new();
            info!("key:{:?} value_cell:{:?}", key, value_cell);
            if let Some(inner_cell) = value_cell.references().first() {
                info!("inner cell:{:?}", inner_cell);

                let bits_remaining = inner_cell.bit_len();
                let bytes_needed = (bits_remaining + 7) / 8;
                let mut string_bytes = vec![0u8; bytes_needed];
                let mut parser = inner_cell.parser();

                parser.load_slice(&mut string_bytes)?;

                let storage_string = String::from_utf8(string_bytes).map_err(|_| {
                    TonCellError::BagOfCellsDeserializationError(
                        "Invalid UTF-8 string in storage location".to_string(),
                    )
                })?;

                info!("Storage_string:{:?} key:{:?}", storage_string, key);
                storage_list.push(storage_string);
            } else {
                return Err(TonCellError::BagOfCellsDeserializationError(
                    "Expected reference in cell but found none".to_string(),
                ));
            }

            storage_locations.insert(key.clone(), storage_list);
        }
        info!("Parsed storage locations: {:?}", storage_locations);
        Ok(storage_locations)
    }

    /// Decodes a Base64 string into a `BagOfCells` and returns the root cell.
    pub fn parse_root_cell_from_boc(boc_base64: &str) -> Result<Arc<Cell>, TonCellError> {
        let boc_bytes = general_purpose::STANDARD.decode(boc_base64).map_err(|_| {
            TonCellError::BagOfCellsDeserializationError(
                "Failed to decode BOC from Base64".to_string(),
            )
        })?;
        println!("boc_bytes:{:?}", boc_bytes);
        info!("boc_bytes:{:?}", boc_bytes);

        let boc = BagOfCells::parse(&boc_bytes)?;
        let root_cell = boc.single_root()?.clone();

        Ok(root_cell)
    }
    pub fn parse_address_from_boc(boc: &str) -> Result<TonAddress, TonCellError> {
        let cell = parse_root_cell_from_boc(boc)?;
        let mut parser = cell.parser();
        let address = parser.load_address()?;

        Ok(address)
    }
    pub fn parse_stack_item_biguint(
        stack: &[StackItem],
        index: usize,
        item_name: &str,
    ) -> ChainResult<BigUint> {
        let item = stack.get(index).ok_or_else(|| {
            ChainCommunicationError::from(HyperlaneTonError::ParsingError(format!(
                "Stack does not contain value at index {} ({})",
                index, item_name
            )))
        })?;

        match &item.value {
            StackValue::String(val) => {
                BigUint::parse_bytes(val.trim_start_matches("0x").as_bytes(), 16).ok_or_else(|| {
                    ChainCommunicationError::from(HyperlaneTonError::ParsingError(format!(
                        "Failed to parse BigUint from string '{}' for {}",
                        val, item_name
                    )))
                })
            }
            _ => Err(ChainCommunicationError::from(
                HyperlaneTonError::ParsingError(format!(
                    "Unexpected stack value type for {}: {:?}",
                    item_name, item.value
                )),
            )),
        }
    }

    pub fn parse_stack_item_to_u32(stack: &[StackItem], index: usize) -> ChainResult<u32> {
        let stack_item = stack.get(index).ok_or_else(|| {
            ChainCommunicationError::CustomError(format!("No stack item at index {index}"))
        })?;
        let value_str = match &stack_item.value {
            StackValue::String(value) => value,
            _ => {
                return Err(ChainCommunicationError::from(
                    HyperlaneTonError::ParsingError(
                        "Failed to get boc: unexpected data type".to_string(),
                    ),
                ));
            }
        };

        u32::from_str_radix(&value_str[2..], 16).map_err(|_| {
            ChainCommunicationError::CustomError(format!(
                "Failed to parse value at index {}: {:?}",
                index, stack_item.value
            ))
        })
    }
    pub fn parse_stack_item_u32(
        stack: &[StackItem],
        index: usize,
        item_name: &str,
    ) -> ChainResult<u32> {
        let biguint = parse_stack_item_biguint(stack, index, item_name)?;
        biguint.clone().try_into().map_err(|_| {
            ChainCommunicationError::CustomError(format!(
                "Value at index {} ({}) is too large for u32: {:?}",
                index, item_name, biguint
            ))
        })
    }

    pub fn parse_eth_address_to_h160(address: &str) -> Result<H160, HyperlaneTonError> {
        let trimmed_address = address.trim_start_matches("0x");
        if trimmed_address.len() != 40 {
            return Err(HyperlaneTonError::ConversionFailed(
                "Invalid Ethereum address length".to_string(),
            ));
        }

        let bytes = hex::decode(trimmed_address).map_err(|e| {
            HyperlaneTonError::ConversionFailed(format!("Failed to decode address: {}", e))
        })?;

        if bytes.len() != 20 {
            return Err(HyperlaneTonError::ConversionFailed(
                "Decoded address does not have 20 bytes (expected for H160)".to_string(),
            ));
        }

        Ok(H160::from_slice(&bytes))
    }
    pub fn parse_transaction(transaction: &Transaction) -> Result<TxnInfo, HyperlaneTonError> {
        let decoded_hash = general_purpose::STANDARD
            .decode(&transaction.hash)
            .map_err(|err| {
                warn!("Failed to decode base64 transaction hash: {:?}", err);
                HyperlaneTonError::ParsingError(format!(
                    "Invalid base64 encoding for transaction hash: {:?}",
                    err
                ))
            })?;

        let txn_hash = if decoded_hash.len() == 32 {
            H512::from_slice(
                &[0u8; 32]
                    .iter()
                    .chain(decoded_hash.iter())
                    .cloned()
                    .collect::<Vec<u8>>(),
            )
        } else {
            warn!(
                "Unexpected hash length: expected 32 bytes, got {}",
                decoded_hash.len()
            );
            return Err(HyperlaneTonError::ParsingError(
                "Decoded transaction hash has unexpected length".to_string(),
            ));
        };

        let gas_limit =
            U256::from_dec_str(&transaction.description.compute_ph.gas_limit).unwrap_or_default();

        let nonce = transaction.lt.parse::<u64>().unwrap_or(0);

        let sender_hex = transaction.account.strip_prefix("0:").ok_or_else(|| {
            warn!(
                "Sender address does not start with '0:': {:?}",
                transaction.account
            );
            HyperlaneTonError::ParsingError("Sender address is invalid".to_string())
        })?;

        let sender =
            H256::from_slice(&hex::decode(sender_hex).map_err(|_| {
                HyperlaneTonError::ParsingError("Failed to decode sender".to_string())
            })?);

        let recipient = transaction.in_msg.as_ref().and_then(|msg| {
            if let Some(dest) = msg.destination.strip_prefix("0:") {
                match hex::decode(dest) {
                    Ok(decoded) if decoded.len() == 32 => Some(H256::from_slice(&decoded)),
                    Ok(_) => {
                        warn!("Recipient address has unexpected length: {:?}", dest);
                        None
                    }
                    Err(e) => {
                        warn!("Failed to decode recipient address '{}': {:?}", dest, e);
                        None
                    }
                }
            } else {
                warn!("Recipient address format is invalid: {:?}", msg.destination);
                None
            }
        });

        let gas_used =
            U256::from_dec_str(&transaction.description.compute_ph.gas_used).unwrap_or_default();

        let receipt = Some(TxnReceiptInfo {
            gas_used,
            cumulative_gas_used: U256::zero(),
            effective_gas_price: None,
        });

        let txn_info = TxnInfo {
            hash: txn_hash,
            gas_limit,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            gas_price: None,
            nonce,
            sender,
            recipient,
            receipt,
            raw_input_data: None,
        };

        Ok(txn_info)
    }
}

#[cfg(test)]
mod tests {
    use crate::run_get_method::{StackItem, StackValue};
    use hyperlane_core::H160;
    use num_bigint::BigUint;

    use crate::utils::parsers::parsers::*;
    #[test]
    fn test_parse_address_from_boc() {
        let address = parse_address_from_boc(
            "te6cckEBAQEAJAAAQ4AK6ETsEZndZnPkJ4gUxnX2otydTPtek+fiTAQfLC3C0JAaOf4x",
        )
        .expect("failed");

        assert_eq!(
            address.to_base64_std(),
            "EQBXQidgjM7rM58hPECmM6+1FuTqZ9r0nz8SYCD5YW4WhHCM".to_string()
        );
    }

    #[test]
    fn test_parse_root_cell_from_boc() {
        let boc_base64 = "te6cckEBAgEANwABQ6AAAAAAAAAAAAAAAAABcqZ6QdO0UVZJKOpooNx6WOrpGnABACBzdG9yYWdlIGxvY2F0aW9u3GbBUg==";
        let root_cell =
            parse_root_cell_from_boc(boc_base64).expect("Failed to parse root cell from BOC");

        // Ensure the root cell is parsed correctly
        assert!(root_cell.bit_len() > 0);
    }

    #[test]
    fn test_parse_root_cell_from_boc_valid() {
        let boc_base64 = "te6cckEBAgEATQABQ6AGvFm965B0z/96EKlW2xGIv+qjfKDHQWY2NlXkdJEINtABAEz3CxqLkN5V+jk24kdOlIIhNfGZYWH0y0ato9U/6pMBogAAAAAAAZnmUE8="; // Example BOC with one root cell
        let result = parse_root_cell_from_boc(boc_base64);
        assert!(result.is_ok());
    }
    #[test]
    fn test_parse_root_cell_from_boc_invalid_base64() {
        let boc_base64 = "invalid_base64";
        let result = parse_root_cell_from_boc(boc_base64);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_eth_address_to_h160_valid() {
        let eth_address = "0x1234567890abcdef1234567890abcdef12345678";

        let h160 = parse_eth_address_to_h160(eth_address).unwrap();

        assert_eq!(
            h160,
            H160::from_slice(&hex::decode(&eth_address[2..]).unwrap())
        );
    }
    #[test]
    fn test_parse_eth_address_to_h160_invalid_length() {
        let eth_address = "0x123456";

        let result = parse_eth_address_to_h160(eth_address);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Conversion data failed");
    }

    #[test]
    fn test_parse_stack_item_to_u32_valid() {
        let stack = vec![StackItem {
            r#type: "cell".to_string(),
            value: StackValue::String("0x0000002a".to_string()),
        }];

        let value = parse_stack_item_to_u32(&stack, 0).unwrap();

        assert_eq!(value, 42);
    }

    #[test]
    fn test_parse_stack_item_to_u32_invalid_index() {
        let stack = vec![StackItem {
            r#type: "cell".to_string(),
            value: StackValue::String("0x0000002a".to_string()),
        }];

        let result = parse_stack_item_to_u32(&stack, 1);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No stack item at index 1");
    }
    #[test]
    fn test_parse_stack_item_biguint_valid() {
        let stack = vec![StackItem {
            r#type: "string".to_string(),
            value: StackValue::String("0x123abc".to_string()),
        }];

        let result = parse_stack_item_biguint(&stack, 0, "test_item");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::from(0x123abc_u32));
    }
    #[test]
    fn test_parse_stack_item_biguint_invalid_index() {
        let stack: Vec<StackItem> = vec![];
        let result = parse_stack_item_biguint(&stack, 0, "test_item");

        assert!(result.is_err());
    }
}
