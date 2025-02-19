import {
  Address,
  Cell,
  Contract,
  ContractProvider,
  Dictionary,
  SendMode,
  Sender,
  beginCell,
  contractAddress,
} from '@ton/core';

import { buildHookMetadataCell } from './utils/builders';
import { OpCodes } from './utils/constants';
import { THookMetadata } from './utils/types';

export type TokenRouterConfig = {
  ismAddress?: Address;
  jettonAddress: Address;
  mailboxAddress: Address;
  // domain -> router address (h256)
  routers: Dictionary<number, Buffer>;
  ownerAddress: Address;
  jettonWalletCode?: Cell;
};

export function tokenRouterConfigToCell(config: TokenRouterConfig): Cell {
  const addrNone = beginCell().storeUint(0, 2);
  const addrStd = beginCell().storeAddress(config.ismAddress);
  return beginCell()
    .storeBuilder(config.ismAddress ? addrStd : addrNone)
    .storeAddress(config.jettonAddress)
    .storeAddress(config.mailboxAddress)
    .storeDict(config.routers)
    .storeMaybeRef(config.jettonWalletCode)
    .storeRef(beginCell().storeAddress(config.ownerAddress).endCell())
    .endCell();
}

export class TokenRouter implements Contract {
  constructor(
    readonly address: Address,
    readonly init?: { code: Cell; data: Cell },
  ) {}

  static createFromAddress(address: Address) {
    return new TokenRouter(address);
  }

  static createFromConfig(
    config: TokenRouterConfig,
    code: Cell,
    workchain = 0,
  ) {
    const data = tokenRouterConfigToCell(config);
    const init = { code, data };
    return new TokenRouter(contractAddress(workchain, init), init);
  }

  async sendDeploy(provider: ContractProvider, via: Sender, value: bigint) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell().endCell(),
    });
  }

  async sendGetIsm(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    queryId: bigint,
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.GET_ISM, 32)
        .storeUint(queryId ?? 0, 64)
        .endCell(),
    });
  }

  async sendHandle(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      queryId: bigint;
      origin: number;
      sender: Buffer; // h256
      messageBody: Cell;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.HANDLE, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeUint(opts.origin, 32)
        .storeBuffer(opts.sender, 32)
        .storeRef(opts.messageBody)
        .endCell(),
    });
  }

  async sendTransferRemote(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      queryId?: bigint;
      destination: number;
      recipient: Buffer;
      amount: bigint;
      hookMetadata?: THookMetadata;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.TRANSFER_REMOTE, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeUint(opts.destination, 32)
        .storeBuffer(opts.recipient, 32)
        .storeUint(opts.amount, 256)
        .storeMaybeRef(
          opts.hookMetadata ? buildHookMetadataCell(opts.hookMetadata!) : null,
        )
        .storeMaybeRef(null)
        .endCell(),
    });
  }
}
