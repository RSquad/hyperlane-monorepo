import {
  Address,
  Builder,
  Cell,
  Contract,
  ContractProvider,
  Dictionary,
  DictionaryKey,
  DictionaryValue,
  SendMode,
  Sender,
  Slice,
  beginCell,
  contractAddress,
} from '@ton/core';

import {
  buildHookMetadataCell,
  buildMessageCell,
  buildMetadataCell,
  readHookMetadataCell,
  readMessageCell,
} from './utils/builders';
import { OpCodes, answer } from './utils/constants';
import {
  THookMetadata,
  TMailboxContractConfig,
  TMessage,
  TMultisigMetadata,
  TProcessRequest,
} from './utils/types';

export const MAILBOX_VERSION = 3;

export function mailboxConfigToCell(config: TMailboxContractConfig): Cell {
  const hooks = beginCell()
    .storeAddress(config.defaultIsm)
    .storeAddress(config.defaultHookAddr)
    .storeAddress(config.requiredHookAddr)
    .endCell();
  return beginCell()
    .storeUint(config.version, 8)
    .storeUint(config.localDomain, 32)
    .storeUint(config.nonce, 32)
    .storeUint(config.latestDispatchedId, 256)
    .storeAddress(config.owner)
    .storeRef(config.deliveryCode)
    .storeDict(
      config.processRequests,
      Mailbox.DeliveryKey,
      Mailbox.DeliveryValue,
    )
    .storeRef(hooks)
    .endCell();
}

export class Mailbox implements Contract {
  constructor(
    readonly address: Address,
    readonly init?: { code: Cell; data: Cell },
  ) {}

  static version = MAILBOX_VERSION;
  static DeliveryKey: DictionaryKey<bigint> = Dictionary.Keys.BigUint(64);
  static DeliveryValue: DictionaryValue<TProcessRequest> = {
    serialize: (src: TProcessRequest, builder: Builder) => {
      const delivery_cell = beginCell()
        .storeAddress(src.initiator)
        .storeAddress(src.ism)
        .storeRef(buildMessageCell(src.message))
        .storeRef(buildHookMetadataCell(src.metadata))
        .endCell();
      builder.storeRef(delivery_cell);
    },
    parse: (src: Slice): TProcessRequest => {
      src = src.loadRef().beginParse();
      const data: TProcessRequest = {
        initiator: src.loadAddress(),
        ism: src.loadAddress(),
        message: readMessageCell(src.loadRef()),
        metadata: readHookMetadataCell(src.loadRef()),
      };
      return data;
    },
  };

  static createFromAddress(address: Address) {
    return new Mailbox(address);
  }

  static createFromConfig(
    config: TMailboxContractConfig,
    code: Cell,
    workchain = 0,
  ) {
    const data = mailboxConfigToCell(config);
    const init = { code, data };
    return new Mailbox(contractAddress(workchain, init), init);
  }

  async sendDeploy(provider: ContractProvider, via: Sender, value: bigint) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell().endCell(),
    });
  }

  async sendDispatch(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      destDomain: number;
      recipientAddr: Buffer;
      message: Cell;
      hookMetadata: THookMetadata;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.DISPATCH, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeUint(opts.destDomain, 32)
        .storeBuffer(opts.recipientAddr)
        .storeRef(opts.message)
        .storeMaybeRef(buildHookMetadataCell(opts.hookMetadata))
        .endCell(),
    });
  }

  async sendProcess(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      metadata: TMultisigMetadata;
      message: TMessage;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.PROCESS, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeRef(buildMessageCell(opts.message))
        .storeMaybeRef(buildMetadataCell(opts.metadata))
        .endCell(),
    });
  }

  async sendIsmVerifyAnswer(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      metadata: TMultisigMetadata;
      message: TMessage;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(answer(OpCodes.VERIFY), 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeBit(false)
        .storeRef(buildMessageCell(opts.message))
        .storeRef(buildMetadataCell(opts.metadata))
        .endCell(),
    });
  }

  async sendGetIsmAnswer(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      metadata: TMultisigMetadata;
      message: TMessage;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(answer(OpCodes.GET_ISM), 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeBit(false)
        .storeRef(buildMessageCell(opts.message))
        .storeRef(buildMetadataCell(opts.metadata))
        .endCell(),
    });
  }

  async sendSetDefaultIsm(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      defaultIsmAddr: Address;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.SET_DEFAULT_ISM, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeAddress(opts.defaultIsmAddr)
        .endCell(),
    });
  }

  async sendSetDefaultHook(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      defaultHookAddr: Address;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.SET_DEFAULT_HOOK, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeAddress(opts.defaultHookAddr)
        .endCell(),
    });
  }

  async sendSetRequiredHook(
    provider: ContractProvider,
    via: Sender,
    value: bigint,
    opts: {
      requiredHookAddr: Address;
      queryId?: number;
    },
  ) {
    await provider.internal(via, {
      value,
      sendMode: SendMode.PAY_GAS_SEPARATELY,
      body: beginCell()
        .storeUint(OpCodes.SET_REQUIRED_HOOK, 32)
        .storeUint(opts.queryId ?? 0, 64)
        .storeAddress(opts.requiredHookAddr)
        .endCell(),
    });
  }

  async getLocalDomain(provider: ContractProvider) {
    const result = await provider.get('get_local_domain', []);
    return result.stack.readNumber();
  }

  async getLatestDispatchedId(provider: ContractProvider) {
    const result = await provider.get('get_latest_dispatched_id', []);
    return result.stack.readNumber();
  }

  async getDefaultIsm(provider: ContractProvider) {
    const result = await provider.get('get_default_ism', []);
    return result.stack.readAddress();
  }

  async getDefaultHook(provider: ContractProvider) {
    const result = await provider.get('get_default_hook', []);
    return result.stack.readAddress();
  }

  async getRequiredHook(provider: ContractProvider) {
    const result = await provider.get('get_required_hook', []);
    return result.stack.readAddress();
  }
}
