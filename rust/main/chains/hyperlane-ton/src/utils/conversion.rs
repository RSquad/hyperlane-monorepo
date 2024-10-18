use hyperlane_core::{HyperlaneContract, HyperlaneMessage, H256, H512, U256};
use tonlib::cell::ArcCell;
use tonlib::{
    address::TonAddress,
    cell::{Cell, CellBuilder},
};

pub fn hyperlane_message_to_cell(message: &HyperlaneMessage) -> Cell {
    let mut writer = CellBuilder::new();
    let mut cell = writer
        .store_u8(8, message.version)
        .expect("Failed to store version")
        .store_u32(32, message.nonce)
        .expect("Failed to store nonce")
        .store_u32(32, message.origin)
        .expect("Failed to store origin")
        .store_slice(&message.sender.as_bytes())
        .expect("Failed to store sender")
        .store_u32(32, message.destination)
        .expect("Failed to store destination_domain")
        .store_slice(&message.recipient.as_bytes())
        .expect("Failed to store recipient")
        .store_reference(&ArcCell::new(metadata_to_cell(message.body.as_slice())))
        .expect("Failed convert from HyperlaneMessage to Cell")
        .build()
        .expect("");

    cell
}
pub fn metadata_to_cell(metadata: &[u8]) -> Cell {
    let mut writer = CellBuilder::new();
    let cell = writer
        .store_slice(metadata)
        .expect("Failed to store signature")
        .build()
        .expect("Failed to build cell");

    cell
}
