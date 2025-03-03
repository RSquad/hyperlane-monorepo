import { OpenedContract } from '@ton/core';

import { JettonMinterContract } from '../wrappers/JettonMinter';
import { JettonWalletContract } from '../wrappers/JettonWallet';
import { TokenRouter } from '../wrappers/TokenRouter';

export enum TokenStandard {
  Synthetic = 'SYNTHETIC',
  Native = 'NATIVE',
  Collateral = 'COLLATERAL',
}

export type Route = {
  jettonMinter?: OpenedContract<JettonMinterContract>;
  jettonWallet?: OpenedContract<JettonWalletContract>;
  tokenRouter: OpenedContract<TokenRouter>;
};
