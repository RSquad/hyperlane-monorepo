export const METADATA_VARIANT = {
  STANDARD: 1,
};

export const OpCodes = {
  DISPATCH_INIT: 0x884b9dd4,
  PROCESS_INIT: 0xaced8a23,
  HANDLE: 0xb99c08d,
  TRANSFER_REMOTE: 0xdd70fba2,
  QUOTE_DISPATCH: 0x18bf902,
  SET_DEFAULT_ISM: 0xd44d8496,
  SET_DEFAULT_HOOK: 0x8e6c735b,
  SET_REQUIRED_HOOK: 0x2f5451cc,
  POST_DISPATCH_REQUIRED: 0x89f4a643,
  POST_DISPATCH_DEFAULT: 0x8a9fb44c,
  SET_BENEFICIARY: 0xfc3adbc,
  SET_PROTOCOL_FEE: 0xf7240b7a,
  COLLECT_PROTOCOL_FEE: 0xaec506d3,
  VERIFY: 0x3b3cca17,
  SET_ISM: 0x9b6299a8,
  REMOVE_ISM: 0x38552523,
  SET_AUTHORIZED_HOOK: 0x995495a2,
  ANNOUNCE: 0x980b3d44,
  GET_ISM: 0x8f32175,
  CLAIM: 0x13a3ca6,
  TRANSFER_OWNERSHIP: 0x295e75a9,
  SET_DEST_GAS_CONFIG: 0x301bf43f,
  SET_VALIDATORS_AND_THRESHOLD: 0x4dad45ea,
  JETTON_TRANSFER: 0xf8a7ea5,
  JETTON_TRANSFER_NOTIFICATION: 0x7362d09c,
  JETTON_INTERNAL_TRANSFER: 0x178d4519,
  JETTON_EXCESSES: 0xd53276db,
  JETTON_BURN: 0x595f07bc,
  JETTON_BURN_NOTIFICATION: 0x7bdd97de,
  JETTON_MINT: 0x642b7d07,
  JETTON_TOP_UP: 0xd372158c,
  JETTON_CHANGE_ADMIN: 0x6501f354,
  SET_ROUTER: 0xca657447,
};

export const ProcessOpCodes = {
  VERIFY: 0xb33e03ab,
  DELIVER_MESSAGE: 0x22967489,
};

export const Errors = {
  UNKNOWN_OPCODE: 0xffff,
  UNAUTHORIZED_SENDER: 103,
  WRONG_MAILBOX_VERSION: 100,
  WRONG_DEST_DOMAIN: 101,
  MESSAGE_DELIVERED: 102,
  MESSAGE_VERIFICATION_FAILED: 104,
  UNKNOWN_SUB_OP: 105,
  INSUFFICIENT_GAS_PAYMENT: 106,
  WRONG_SIGNATURE: 107,
  WRONG_VALIDATOR: 110,
  PUBKEY_RECOVERY: 111,
  STORAGE_LOCATION_REPLAY: 112,
  DOMAIN_VALIDATORS_NOT_FOUND: 113,
  MSG_VALUE_TOO_LOW: 114,
  MERKLE_TREE_FULL: 115,
  EXCEEDS_MAX_PROTOCOL_FEE: 116,
  INSUFFICIENT_PROTOCOL_FEE: 117,
};

export const ANSWER_BIT = 0x80000000;
