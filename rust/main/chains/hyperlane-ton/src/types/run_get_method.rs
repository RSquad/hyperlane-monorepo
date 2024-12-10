use derive_new::new;
use serde::{Deserialize, Serialize};

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RunGetMethodResponse {
//     pub address: String,
//     pub method: String,
//     pub stack: Vec<StackItem>,
// }

#[derive(Deserialize, Debug)]
pub struct RunGetMethodResponse {
    pub gas_used: u64,
    pub exit_code: i32,
    pub stack: Vec<StackItem>,
}

#[derive(Debug, Serialize, Deserialize, new)]
pub struct StackItem {
    #[serde(rename = "type")]
    pub r#type: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub error: String,
}

#[derive(Debug)]
pub enum GetMethodResponse {
    Success(RunGetMethodResponse),
    Error(ErrorResponse),
}

impl GetMethodResponse {
    pub fn from_json(json: &str) -> Result<GetMethodResponse, serde_json::Error> {
        match serde_json::from_str::<RunGetMethodResponse>(json) {
            Ok(success) => Ok(GetMethodResponse::Success(success)),
            Err(_) => {
                let err = serde_json::from_str::<ErrorResponse>(json)?;
                Ok(GetMethodResponse::Error(err))
            }
        }
    }
}
