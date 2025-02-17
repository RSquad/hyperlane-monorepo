import {
  Address,
  Cell,
  Contract,
  ContractProvider,
  SendMode,
  Sender,
  beginCell,
  contractAddress,
  fromNano,
} from '@ton/core';

import { buildHookMetadataCell, buildMessageCell } from './utils/builders';
import { OpCodes } from './utils/constants';
import { THookMetadata, TMessage } from './utils/types';

export type ProtocolFeeHookConfig = {
  protocolFee: bigint;
  maxProtocolFee: bigint;
  beneficiary: Address;
  owner: Address;
};

export function protocolFeeHookConfigToCell(
  config: ProtocolFeeHookConfig,
): Cell {
  return beginCell()
    .storeUint(config.protocolFee, 128)
    .storeUint(config.maxProtocolFee, 128)
    .storeUint(0, 128) // collected fees
    .storeAddress(config.beneficiary)
    .storeAddress(config.owner)
    .endCell();
}

export class ProtocolFeeHook implements Contract {
  constructor(
    readonly address: Address,
    readonly init?: { code: Cell; data: Cell },
  ) {}

  static createFromAddress(address: Address) {
    return new ProtocolFeeHook(address);
  }

  static createFromConfig(
    config: ProtocolFeeHookConfig,
    code: Cell,
    workchain = 0,
  ) {
    const data = protocolFeeHookConfigToCell(config);
    const init = { code, data };
    return new ProtocolFeeHook(contractAddress(workchain, init), init);
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
        .storeRef(buildHookMetadataCell(opts.hookMetadata))
        .storeRef(buildMessageCell(opts.message))
        .endCell(),
    });
  }

  async sendCollectProtocolFee(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    queryId?: number,
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.COLLECT_PROTOCOL_FEE, 32)
        .storeUint(queryId ?? 0, 64)
        .endCell(),
    });
  }

  async sendSetProtocolFee(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      protocolFee: bigint;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.SET_PROTOCOL_FEE, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeUint(opts.protocolFee, 128)
        .endCell(),
    });
  }

  async sendSetBeneficiary(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      beneficiaryAddr: Address;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.SET_BENEFICIARY, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeAddress(opts.beneficiaryAddr)
        .endCell(),
    });
  }

  async sendTransferOwnership(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      ownerAddr: Address;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.TRANSFER_OWNERSHIP, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeAddress(opts.ownerAddr)
        .endCell(),
    });
  }

  async getProtocolFee(provider: ContractProvider) {
    const result = await provider.get('get_hook_data', []);
    result.stack.skip();
    return result.stack.readBigNumber();
  }

  async getMaxProtocolFee(provider: ContractProvider) {
    const result = await provider.get('get_hook_data', []);
    return result.stack.readBigNumber();
  }

  async getBeneficiary(provider: ContractProvider) {
    const result = await provider.get('get_hook_data', []);
    result.stack.skip(2);
    return result.stack.readAddress();
  }

  async getHookType(provider: ContractProvider) {
    const result = await provider.get('get_hook_data', []);
    result.stack.skip(3);
    return result.stack.readNumber();
  }

  async getBalance(provider: ContractProvider) {
    return fromNano((await provider.getState()).balance);
  }
}
