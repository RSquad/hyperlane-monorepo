import { Address, Cell, Dictionary } from '@ton/core';
import { Signature } from 'ethers';

export type THookMetadata = {
  variant: number;
  msgValue: bigint;
  gasLimit: bigint;
  refundAddress: Buffer;
};

export type TGasConfig = {
  gasOracle: bigint;
  gasOverhead: bigint;
  exchangeRate: bigint;
  gasPrice: bigint;
};

export type TSignature = {
  s: bigint;
  v: bigint;
  r: bigint;
};

export type TMultisigMetadata = {
  originMerkleHook: Buffer;
  root: Buffer;
  index: bigint;
  signatures: TSignature[];
};

export type TMessage = {
  version: number;
  nonce: number;
  origin: number;
  sender: Buffer;
  destination: number;
  recipient: Buffer;
  body: Cell;
};

export type TProcessRequest = {
  message: TMessage;
  metadata: THookMetadata;
  initiator: Address;
  ism: Address;
};

export type TMailboxContractConfig = {
  version: number;
  localDomain: number;
  nonce: number;
  latestDispatchedId: bigint;
  defaultIsm: Address;
  defaultHookAddr: Address;
  requiredHookAddr: Address;
  owner: Address;
  deliveryCode: Cell;
  processRequests: Dictionary<bigint, TProcessRequest>;
};

export type TJettonWalletContractConfig = {
  ownerAddress: Address;
  minterAddress: Address;
};

export type TJettonMinterContractConfig = {
  adminAddress: Address;
  content: Cell;
  jettonWalletCode: Cell;
};
