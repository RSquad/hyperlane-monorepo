use anyhow::Error;
use hex::FromHex;
use hyperlane_core::{HyperlaneMessage, H160, H256, H512, U256};
use log::info;
use num_bigint::BigUint;
use std::collections::HashMap;
use std::sync::Arc;

use tonlib_core::cell::dict::predefined_readers::{key_reader_uint, val_reader_cell};
use tonlib_core::cell::{ArcCell, BagOfCells, Cell, CellBuilder, TonCellError};
use tonlib_core::{TonAddress, TonHash};

pub struct ConversionUtils;

impl ConversionUtils {
    pub fn base64_to_h512(hash: &str) -> Result<H512, Error> {
        let decoded = base64::decode(hash)
            .map_err(|e| Error::msg(format!("Failed to decode base64 hash: {}", e)))?;
        if decoded.len() > 64 {
            return Err(Error::msg("Decoded hash length exceeds 64 bytes"));
        }
        let mut padded = [0u8; 64];
        padded[..decoded.len()].copy_from_slice(&decoded);

        Ok(H512::from_slice(&padded))
    }

    pub fn metadata_to_cell(metadata: &[u8]) -> Result<Cell, anyhow::Error> {
        let mut writer = CellBuilder::new();
        let cell = writer
            .store_slice(metadata)
            .expect("Failed to store signature")
            .build()
            .expect("Failed build");

        Ok(cell)
    }
    pub fn hyperlane_message_to_message(
        message: &HyperlaneMessage,
    ) -> Result<Message, anyhow::Error> {
        let sender_bytes = message
            .sender
            .as_bytes()
            .to_vec()
            .iter()
            .skip_while(|&&x| x == 0)
            .copied()
            .collect();
        info!("sender_bytes:{:?}", sender_bytes);
        let recipient = message.recipient.as_bytes().to_vec();
        info!("recipient:{:?}", recipient);

        let id: usize = 27; // needed check

        let mut writer = CellBuilder::new();
        let body = writer
            .store_slice(message.body.as_slice())
            .expect("Failed to store_slice")
            .build()
            .expect("Failed to build");

        Ok(Message {
            id: BigUint::from(id),
            version: message.version,
            nonce: message.nonce,
            origin: message.origin,
            sender: sender_bytes,
            destination_domain: 0,
            recipient,
            body,
        })
    }

    /// Creates a linked list of cells, each containing up to 6 addresses.
    /// If there are more than 6 addresses, the next cell is created with a reference to the previous cell.
    pub fn create_address_linked_cells(addresses: &[H160]) -> Result<Cell, TonCellError> {
        let mut remaining_addresses = addresses;
        let mut current_cell = CellBuilder::new();

        loop {
            let addresses_in_cell = remaining_addresses.len().min(6);
            //let remaining_count = remaining_addresses.len() - addresses_in_cell;

            info!(
                "Creating a new cell segment with {} addresses.",
                addresses_in_cell
            );

            // Write down the number of addresses in the current cell
            current_cell.store_u8(8, addresses_in_cell as u8)?;

            // We write down the addresses ourselves
            for address in &remaining_addresses[..addresses_in_cell] {
                info!("Storing address: {:?}", address);
                current_cell.store_uint(160, &BigUint::from_bytes_be(address.as_bytes()))?;
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

        //let dict_cell = root_cell.clone();
        let parsed = root_cell
            .parser()
            .load_dict(256, key_reader_uint, val_reader_cell)?;

        for (key, value_cell) in &parsed {
            let mut storage_list = Vec::new();

            if let Some(inner_cell) = value_cell.references().first() {
                let mut parser = inner_cell.parser();

                let bits_remaining = parser.remaining_bits();
                let bytes_needed = (bits_remaining + 7) / 8;
                let mut string_bytes = vec![0u8; bytes_needed];

                parser.load_slice(&mut string_bytes)?;

                let storage_string = String::from_utf8(string_bytes).map_err(|_| {
                    TonCellError::BagOfCellsDeserializationError(
                        "Invalid UTF-8 string in storage location".to_string(),
                    )
                })?;

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
        let boc_bytes = base64::decode(boc_base64).map_err(|_| {
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
        info!("Parsed address from BOC: {:?}", address);
        Ok(address)
    }
    pub fn ton_address_to_h256(address: &TonAddress) -> Result<H256, String> {
        let address_hex = address.to_hex();
        let clean_hex = address_hex.trim_start_matches(&format!("{}:", address.workchain));

        let bytes =
            <[u8; 32]>::from_hex(clean_hex).map_err(|e| format!("Failed to parse hex: {:?}", e))?;

        info!("H256: {:?}", H256::from(bytes));
        Ok(H256::from(bytes))
    }

    pub fn u256_to_biguint(value: U256) -> BigUint {
        let mut bytes = [0u8; 32]; // 256 bit = 32 byte
        value.to_little_endian(&mut bytes);
        BigUint::from_bytes_le(&bytes)
    }
    pub fn h256_to_ton_address(h256: &H256, workchain: i32) -> Result<TonAddress, String> {
        let h256_str = format!("{:x}", h256);

        let h256_hex = h256_str.trim_start_matches("0x");
        info!("H256_hex:{:?}", h256_hex);

        let bytes: TonHash = hex::decode(h256_hex)
            .map_err(|e| format!("Failed to decode H256 hex to bytes: {:?}", e))?
            .as_slice()
            .try_into()
            .map_err(|e| format!("Failed to convert decoded bytes into TonHash: {:?}", e))?;

        let addr = TonAddress::new(workchain, &bytes);

        Ok(addr)
    }

    pub fn parse_data_cell(_data: &ArcCell) -> Result<HyperlaneMessage, Error> {
        todo!();
    }
}
pub struct Metadata {
    pub signature: [u8; 64],
}

impl Metadata {
    pub fn new(signature: [u8; 64]) -> Self {
        Self { signature }
    }

    pub fn to_cell(&self) -> Cell {
        let mut writer = CellBuilder::new();
        let cell = writer
            .store_slice(&self.signature)
            .expect("Failed to store signature")
            .build()
            .expect("");

        cell
    }
}
#[derive(Debug, PartialEq)]
pub struct Message {
    pub id: BigUint,
    pub version: u8,
    pub nonce: u32,
    pub origin: u32,
    pub sender: Vec<u8>,
    pub destination_domain: u8,
    pub recipient: Vec<u8>,
    pub body: Cell,
}

impl Message {
    pub fn new(
        id: BigUint,
        version: u8,
        nonce: u32,
        origin: u32,
        sender: Vec<u8>,
        destination_domain: u8,
        recipient: Vec<u8>,
        body: Cell,
    ) -> Self {
        Self {
            id,
            version,
            nonce,
            origin,
            sender,
            destination_domain,
            recipient,
            body,
        }
    }

    pub fn to_cell(&self) -> Cell {
        let mut writer = CellBuilder::new();

        let cell = writer
            .store_uint(256, &self.id)
            .expect("")
            .store_uint(8, &BigUint::from(self.version))
            .expect("Failed to store version")
            .store_uint(32, &BigUint::from(self.nonce))
            .expect("Failed to store nonce")
            .store_uint(32, &BigUint::from(self.origin.clone()))
            .expect("Failed to store origin")
            .store_uint(256, &BigUint::from_bytes_be(self.sender.as_slice()))
            .expect("Failed to store sender")
            .store_uint(8, &BigUint::from(self.destination_domain.clone()))
            .expect("Failed to store destination_domain")
            .store_uint(256, &BigUint::from_bytes_be(self.recipient.as_slice()))
            .expect("Failed to store recipient")
            .store_reference(&ArcCell::new(self.body.clone()))
            .expect("")
            .build()
            .expect("");

        cell
    }
}

#[cfg(test)]
mod tests {
    use super::ConversionUtils;
    use hyperlane_core::{H160, H512, U256};
    use num_bigint::BigUint;
    use num_traits::Zero;

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
    fn test_create_address_linked_cells() {
        let addresses = vec![
            H160::from_low_u64_be(0x12345678),
            H160::from_low_u64_be(0x87654321),
        ];
        let cell = ConversionUtils::create_address_linked_cells(&addresses)
            .expect("Failed to create linked cells");

        // Ensure the cell is created with the expected number of addresses
        assert_eq!(cell.references().len(), 1);
    }
    fn test_create_8_addresses_linked_cells() {
        let addresses: Vec<H160> = vec![
            H160::from_low_u64_be(0x1234567890abcdef),
            H160::from_low_u64_be(0xabcdef1234567890),
            H160::from_low_u64_be(0x9876543210fedcba),
            H160::from_low_u64_be(0x0004567890abcdef),
            H160::from_low_u64_be(0x0000000000000890),
            H160::from_low_u64_be(0x0000000000f0dcba),
            H160::from_low_u64_be(0x0000000000000001),
            H160::from_low_u64_be(0x0000000000000002),
        ];

        let cell = ConversionUtils::create_address_linked_cells(addresses.as_slice()).unwrap();

        assert_eq!(cell.bit_len(), 968);
        let arr = [
            6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 52, 86, 120, 144, 171, 205, 239, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 171, 205, 239, 18, 52, 86, 120, 144, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 152, 118, 84, 50, 16, 254, 220, 186, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4,
            86, 120, 144, 171, 205, 239, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,
            144, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 220, 186,
        ];
        assert_eq!(cell.data(), arr);
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
