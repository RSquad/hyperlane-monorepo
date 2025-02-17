import {
  Address,
  Cell,
  Contract,
  ContractProvider,
  SendMode,
  Sender,
  beginCell,
  contractAddress,
} from '@ton/core';

import { buildHookMetadataCell } from './utils/builders';
import { OpCodes } from './utils/constants';
import { THookMetadata } from './utils/types';

export type TokenCollateralConfig = {
  ismAddress?: Address;
  jettonAddress: Address;
  mailboxAddress: Address;
};

export function tokenCollateralConfigToCell(
  config: TokenCollateralConfig,
): Cell {
  const addrNone = beginCell().storeUint(0, 2);
  const addrStd = beginCell().storeAddress(config.ismAddress);
  return beginCell()
    .storeBuilder(config.ismAddress ? addrStd : addrNone)
    .storeAddress(config.jettonAddress)
    .storeAddress(config.mailboxAddress)
    .endCell();
}

export class TokenCollateral implements Contract {
  constructor(
    readonly address: Address,
    readonly init?: { code: Cell; data: Cell },
  ) {}

  static createFromAddress(address: Address) {
    return new TokenCollateral(address);
  }

  static createFromConfig(
    config: TokenCollateralConfig,
    code: Cell,
    workchain = 0,
  ) {
    const data = tokenCollateralConfigToCell(config);
    const init = { code, data };
    return new TokenCollateral(contractAddress(workchain, init), init);
  }

  async sendDeploy(provider: ContractProvider, via: Sender, value: bigint) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell().endCell(),
    });
  }

  async sendHandle(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      queryId: bigint;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.HANDLE, 32)
        .storeUint(opts.queryId ?? 0, 64)

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
      value: value + opts.amount,
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
