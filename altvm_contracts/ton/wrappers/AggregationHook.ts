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

import { buildHookMetadataCell, buildMessageCell } from './utils/builders';
import { OpCodes } from './utils/constants';
import { THookMetadata, TMessage } from './utils/types';

export type AggregationHookConfig = {
  mailboxAddr: Address;
};

export function aggregationHookConfigToCell(
  config: AggregationHookConfig,
): Cell {
  return beginCell().storeAddress(config.mailboxAddr).endCell();
}

export class AggregationHook implements Contract {
  constructor(
    readonly address: Address,
    readonly init?: { code: Cell; data: Cell },
  ) {}

  static createFromAddress(address: Address) {
    return new AggregationHook(address);
  }

  static createFromConfig(
    config: AggregationHookConfig,
    code: Cell,
    workchain = 0,
  ) {
    const data = aggregationHookConfigToCell(config);
    const init = { code, data };
    return new AggregationHook(contractAddress(workchain, init), init);
  }

  async sendDeploy(provider: ContractProvider, via: Sender, value: bigint) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell().endCell(),
    });
  }

  async sendPostDispatch(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      message: TMessage;
      hookMetadata: THookMetadata;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.POST_DISPATCH, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeRef(buildMessageCell(opts.message))
        .storeMaybeRef(buildHookMetadataCell(opts.hookMetadata))
        .endCell(),
    });
  }
}
