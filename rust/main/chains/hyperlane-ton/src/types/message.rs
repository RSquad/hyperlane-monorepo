use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub address_book: AddressBook,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressBook {
    #[serde(flatten)]
    pub props: std::collections::HashMap<String, AddressProp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressProp {
    pub user_friendly: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub bounce: bool,
    pub bounced: bool,
    pub created_at: String,
    pub created_lt: String,
    pub destination: String,
    pub fwd_fee: String,
    pub hash: String,
    pub ihr_disabled: bool,
    pub ihr_fee: String,
    pub import_fee: String,
    pub init_state: Option<MessageState>,
    pub message_content: Option<MessageContent>,
    pub opcode: i32,
    pub source: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageState {
    pub body: String,
    pub decoded: Option<DecodedMessage>,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageContent {
    pub body: String,
    pub decoded: Option<DecodedMessage>,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecodedMessage {
    pub comment: String,
    pub r#type: String,
}
