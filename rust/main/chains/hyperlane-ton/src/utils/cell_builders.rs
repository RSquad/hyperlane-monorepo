pub mod cell_builders {
    use num_bigint::BigUint;
    use std::{collections::HashMap, sync::Arc};
    use tonlib_core::cell::{ArcCell, Cell, CellBuilder, TonCellError};
    use tracing::info;

    use hyperlane_core::{HyperlaneMessage, H256};

    use crate::t_metadata::TMetadata;

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
            .store_uint(32, &BigUint::from(tmetadata.index))
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store index: {:?}", e))
            })?;

        let mut signature_dict = HashMap::new();

        for (key, signature) in &tmetadata.signatures {
            let mut signature_builder = CellBuilder::new();
            if signature.len() != 65 {
                return Err(TonCellError::CellBuilderError(format!(
                    "Invalid signature length: expected 65 bytes, got {}",
                    signature.len()
                )));
            }

            let r = BigUint::from_bytes_be(&signature[0..32]);
            let s = BigUint::from_bytes_be(&signature[32..64]);
            let v = signature[64];

            signature_builder.store_u8(8, v).map_err(|_| {
                TonCellError::CellBuilderError("Failed to store 'v' in signature".to_string())
            })?;

            signature_builder.store_uint(256, &r).map_err(|_| {
                TonCellError::CellBuilderError("Failed to store 'r' in signature".to_string())
            })?;

            signature_builder.store_uint(256, &s).map_err(|_| {
                TonCellError::CellBuilderError("Failed to store 's' in signature".to_string())
            })?;

            let signature_cell = signature_builder.build().map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to build signature cell: {:?}", e))
            })?;
            let data = signature_cell.data().to_vec();

            signature_dict.insert(BigUint::from(*key), data);
        }
        let value_writer =
            |builder: &mut CellBuilder, value: Vec<u8>| -> Result<(), TonCellError> {
                builder.store_slice(&value).map_err(|_| {
                    TonCellError::CellBuilderError(format!("Failed to store signature cell"))
                })?;

                Ok(())
            };

        writer
            .store_dict(32, value_writer, signature_dict)
            .map_err(|e| {
                TonCellError::CellBuilderError(format!("Failed to store dictionary: {:?}", e))
            })?;

        let cell = writer.build().map_err(|e| {
            TonCellError::CellBuilderError(format!("Failed to build cell: {:?}", e))
        })?;
        info!("metadata cell:{:?}", cell);

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
                let next_cell = create_address_linked_cells(remaining_addresses)?;
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
}
