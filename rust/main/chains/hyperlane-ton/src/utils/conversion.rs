use std::{collections::HashMap, sync::Arc};

use anyhow::Error;
use base64::{engine::general_purpose, Engine};
use hyperlane_core::{
    ChainCommunicationError, ChainResult, HyperlaneMessage, H160, H256, H512, U256,
};
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use tonlib_core::{
    cell::{
        dict::predefined_readers::{key_reader_uint, val_reader_cell},
        ArcCell, BagOfCells, Cell, CellBuilder, TonCellError,
    },
    TonAddress, TonHash,
};
use tracing::info;

use crate::{
    error::HyperlaneTonError,
    run_get_method::{StackItem, StackValue},
    t_metadata::TMetadata,
};

pub struct ConversionUtils;

impl ConversionUtils {
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

    pub fn metadata_to_cell(metadata: &[u8]) -> Result<Cell, TonCellError> {
        let tmetadata = TMetadata::from_bytes(metadata).unwrap();
        let mut writer = CellBuilder::new();

        writer
            .store_slice(&tmetadata.origin_merkle_hook)
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store metadata slice: {:?}", e))
            })?;
        writer.store_slice(&tmetadata.root).map_err(|e| {
            TonCellError::CellBuilderError(format!("Failed to store root slice: {:?}", e))
        })?;
        writer
            .store_uint(32, &BigUint::from_u32(tmetadata.index).unwrap())
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store index: {:?}", e))
            })?;

        let mut signature_dict = HashMap::new();
        for (key, signature) in &tmetadata.signatures {
            signature_dict.insert(BigUint::from_u32(*key).unwrap(), signature.to_vec());
        }
        let value_writer =
            |builder: &mut CellBuilder, value: Vec<u8>| -> Result<(), TonCellError> {
                builder.store_slice(&value).map(|_| ()).map_err(|_| {
                    TonCellError::CellBuilderError(format!("Failed to store value in dict"))
                })
            };
        writer
            .store_dict(32, value_writer, signature_dict)
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store dictionary: {:?}", e))
            })?;

        let cell = writer.build().map_err(|e| {
            TonCellError::CellBuilderError(format!("Failed to build cell: {:?}", e))
        })?;

        Ok(cell)
    }

    /// Creates a linked list of cells, each containing up to 6 addresses.
    /// If there are more than 6 addresses, the next cell is created with a reference to the previous cell.
    pub fn create_address_linked_cells(addresses: &[H256]) -> Result<Cell, TonCellError> {
        let mut remaining_addresses = addresses;
        let mut current_cell = CellBuilder::new();

        loop {
            let addresses_in_cell = remaining_addresses.len().min(3);
            info!(
                "Creating a new cell segment with {} addresses.",
                addresses_in_cell
            );

            // We write down the addresses ourselves
            for address in &remaining_addresses[..addresses_in_cell] {
                info!("Storing address: {:?}", address);
                current_cell.store_uint(256, &BigUint::from_bytes_be(address.as_bytes()))?;
            }
            remaining_addresses = &remaining_addresses[addresses_in_cell..];

            // If the remaining addresses are greater than 0, create the next cell
            if !remaining_addresses.is_empty() {
                info!("More addresses remaining, creating reference to next cell.");
                let next_cell = ConversionUtils::create_address_linked_cells(remaining_addresses)?;
                current_cell.store_reference(&Arc::new(next_cell))?;
            }
            // We build a cell and return it if only the current addresses remain
            let result_cell = current_cell.build()?;
            info!(
                "Finished creating cell list with root cell hash: {:?}",
                result_cell
            );

            return Ok(result_cell);
        }
    }

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

        let boc = BagOfCells::parse(&boc_bytes)?;
        let root_cell = boc.single_root()?.clone();

        Ok(root_cell)
    }
    /// Parses the first address from a BOC (Bag of Cells) encoded as a Base64 string.
    /// This function decodes the BOC, extracts the root cell, and retrieves the address stored in it.
    pub async fn parse_address_from_boc(boc: &str) -> Result<TonAddress, TonCellError> {
        let cell = Self::parse_root_cell_from_boc(boc)?;
        let mut parser = cell.parser();
        let address = parser.load_address()?;

        Ok(address)
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

    pub fn parse_stack_item_to_u32(stack: &[StackItem], index: usize) -> ChainResult<u32> {
        let stack_item = stack.get(index).ok_or_else(|| {
            ChainCommunicationError::CustomError(format!("No stack item at index {index}"))
        })?;
        let str = match &stack_item.value {
            StackValue::String(value) => value,
            _ => {
                return Err(ChainCommunicationError::from(
                    HyperlaneTonError::ParsingError(
                        "Failed to get boc: unexpected data type".to_string(),
                    ),
                ));
            }
        };

        u32::from_str_radix(&str[2..], 16).map_err(|_| {
            ChainCommunicationError::CustomError(format!(
                "Failed to parse value at index {}: {:?}",
                index, stack_item.value
            ))
        })
    }

    pub fn build_hyperlane_message_cell(message: &HyperlaneMessage) -> Result<Cell, TonCellError> {
        let body = CellBuilder::new()
            .store_slice(message.body.as_slice())
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store body slice: {:?}", e))
            })?
            .build()
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to build body cell: {:?}", e))
            })?;

        let mut writer = CellBuilder::new();

        writer
            .store_uint(8, &BigUint::from(message.version))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store version: {:?}", e))
            })?;
        writer
            .store_uint(32, &BigUint::from(message.nonce))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store nonce: {:?}", e))
            })?;
        writer
            .store_uint(32, &BigUint::from(message.origin))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store origin: {:?}", e))
            })?;
        writer
            .store_uint(256, &BigUint::from_bytes_be(message.sender.as_bytes()))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store sender: {:?}", e))
            })?;
        writer
            .store_uint(32, &BigUint::from(message.destination))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store destination: {:?}", e))
            })?;
        writer
            .store_uint(256, &BigUint::from_bytes_be(message.recipient.as_bytes()))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store recipient: {:?}", e))
            })?;
        writer.store_reference(&ArcCell::new(body)).map_err(|e| {
            TonCellError::CellBuilderError(format!("Failed to store body reference: {:?}", e))
        })?;

        writer
            .build()
            .map_err(|e| TonCellError::CellBuilderError(format!("Failed to build cell: {:?}", e)))
    }
    pub fn parse_stack_item_biguint(
        stack: &[StackItem],
        index: usize,
        item_name: &str,
    ) -> ChainResult<BigUint> {
        let item = stack.get(index).ok_or_else(|| {
            ChainCommunicationError::CustomError(format!(
                "Stack does not contain value at index {} ({})",
                index, item_name
            ))
        })?;

        match &item.value {
            StackValue::String(val) => {
                BigUint::parse_bytes(val.trim_start_matches("0x").as_bytes(), 16).ok_or_else(|| {
                    ChainCommunicationError::CustomError(format!(
                        "Failed to parse BigUint from string '{}' for {}",
                        val, item_name
                    ))
                })
            }
            _ => Err(ChainCommunicationError::CustomError(format!(
                "Unexpected stack value type for {}",
                item_name
            ))),
        }
    }

    pub fn parse_stack_item_u32(
        stack: &[StackItem],
        index: usize,
        item_name: &str,
    ) -> ChainResult<u32> {
        let biguint = Self::parse_stack_item_biguint(stack, index, item_name)?;
        biguint.clone().try_into().map_err(|_| {
            ChainCommunicationError::CustomError(format!(
                "Value at index {} ({}) is too large for u32: {:?}",
                index, item_name, biguint
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use hyperlane_core::{H512, U256};
    use num_bigint::BigUint;
    use num_traits::Zero;

    use super::ConversionUtils;

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

        let result = ConversionUtils::base64_to_h512(hash_str).expect("Conversion failed");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_base64_to_h512_invalid() {
        let invalid_hash_str = "invalid_base64_string";

        let result = ConversionUtils::base64_to_h512(invalid_hash_str);
        assert!(result.is_err(), "Expected an error for invalid input");
    }

    #[test]
    fn test_parse_root_cell_from_boc() {
        let boc_base64 = "te6cckEBAgEANwABQ6AAAAAAAAAAAAAAAAABcqZ6QdO0UVZJKOpooNx6WOrpGnABACBzdG9yYWdlIGxvY2F0aW9u3GbBUg==";
        let root_cell = ConversionUtils::parse_root_cell_from_boc(boc_base64)
            .expect("Failed to parse root cell from BOC");

        // Ensure the root cell is parsed correctly
        assert!(root_cell.bit_len() > 0);
    }
    #[test]
    fn test_u256_to_biguint_zero() {
        // Create a U256 value of 0
        let u256_value = U256::zero();

        // Convert to BigUint
        let biguint_value = ConversionUtils::u256_to_biguint(u256_value);

        // Verify correctness
        assert_eq!(biguint_value, BigUint::zero());
    }
    fn test_u256_to_biguint_conversion() {
        // Create a U256 value
        let u256_value = U256::from_dec_str("1234567890123456789012345678901234567890").unwrap();

        // Convert to BigUint
        let biguint_value = ConversionUtils::u256_to_biguint(u256_value);

        // Verify correctness
        let expected_value =
            BigUint::parse_bytes(b"1234567890123456789012345678901234567890", 10).unwrap();
        assert_eq!(biguint_value, expected_value);
    }
}
