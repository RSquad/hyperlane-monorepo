use anyhow::Error;
use hyperlane_core::{HyperlaneContract, HyperlaneMessage, H256, H512, U256};
use log::info;
use num_bigint::BigUint;
use tonlib::cell::ArcCell;
use tonlib::{
    address::TonAddress,
    cell::{Cell, CellBuilder},
};

pub struct ConversionUtils;

impl ConversionUtils {
    pub fn base64_to_H512(hash: &str) -> Result<H512, Error> {
        let decoded = base64::decode(hash)
            .map_err(|e| Error::msg(format!("Failed to decode base64 hash: {}", e)))?;
        if decoded.len() > 64 {
            return Err(Error::msg("Decoded hash length exceeds 64 bytes"));
        }
        let mut padded = [0u8; 64];
        padded[..decoded.len()].copy_from_slice(&decoded);

        Ok(H512::from_slice(&padded))
    }
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
pub fn hyperlane_message_to_message(message: &HyperlaneMessage) -> Result<Message, anyhow::Error> {
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
    let mut body = writer
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
        recipient: recipient,
        body: body,
    })
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
        let mut cell = writer
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

        let mut cell = writer
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
    use hyperlane_core::{HyperlaneContract, HyperlaneMessage, H256, H512, U256};

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

        let result = ConversionUtils::base64_to_H512(hash_str).expect("Conversion failed");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_base64_to_h512_invalid() {
        let invalid_hash_str = "invalid_base64_string";

        let result = ConversionUtils::base64_to_H512(invalid_hash_str);
        assert!(result.is_err(), "Expected an error for invalid input");
    }
}
