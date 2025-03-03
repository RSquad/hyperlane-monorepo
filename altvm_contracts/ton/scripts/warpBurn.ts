import { NetworkProvider } from '@ton/blueprint';
import { toNano } from '@ton/core';

import { buildTokenMessage } from '../wrappers/utils/builders';
import { METADATA_VARIANT } from '../wrappers/utils/constants';
import { HookMetadata } from '../wrappers/utils/types';

import { loadWarpRoute } from './common';
import { TokenStandard } from './types';

export async function run(provider: NetworkProvider) {
  const originDomain = Number(process.env.ORIGIN_DOMAIN);
  const destDomain = Number(process.env.DESTINATION_DOMAIN);
  const origTokenStandard =
    (process.env.ORIGIN_TOKEN_STANDARD as TokenStandard) ??
    TokenStandard.Synthetic;
  const burnAmount = toNano(process.env.AMOUNT!);

  const route = loadWarpRoute(provider, originDomain);
  console.log(
    `Dispatching (burn) from domain ${originDomain} to ${destDomain}`,
  );

  if (origTokenStandard === TokenStandard.Synthetic) {
    if (!route.jettonMinter) {
      throw new Error('No jetton wallet');
    }
    await route.jettonWallet!.sendBurn(provider.sender(), {
      value: toNano(0.6),
      queryId: 0,
      jettonAmount: burnAmount,
      responseAddress: provider.sender().address!,
      destDomain: destDomain,
      recipientAddr: route.tokenRouter.address.hash,
      message: buildTokenMessage(provider.sender().address!.hash, burnAmount),
      hookMetadata: HookMetadata.fromObj({
        variant: METADATA_VARIANT.STANDARD,
        msgValue: toNano('1'),
        gasLimit: 100000000n,
        refundAddress: provider.sender().address!.hash,
      }).toCell(),
    });
  }
  console.log('DONE');
}
