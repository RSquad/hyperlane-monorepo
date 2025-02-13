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

import { OpCodes } from './utils/constants';

export type TokenCollateralConfig = {};

export function tokenCollateralConfigToCell(
  config: TokenCollateralConfig,
): Cell {
  return beginCell().endCell();
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
}
