import { NetworkProvider } from '@ton/blueprint';
import { Address, toNano } from '@ton/core';

import { loadWarpRoute } from './common';
import { Route, TokenStandard } from './types';

export async function run(provider: NetworkProvider) {
  const originDomain = Number(process.env.ORIGIN_DOMAIN);
  const destDomain = Number(process.env.DESTINATION_DOMAIN);
  const origTokenStandard =
    (process.env.ORIGIN_TOKEN_STANDARD as TokenStandard) ??
    TokenStandard.Native;
  const sendAmount = toNano(process.env.AMOUNT!);
  console.log(`sendAmount: ${sendAmount}`);
  const route = loadWarpRoute(provider, originDomain);
  console.log(`Dispatching from domain ${originDomain} to ${destDomain}`);

  if (origTokenStandard === TokenStandard.Native) {
    await route.tokenRouter.sendTransferRemote(
      provider.sender(),
      sendAmount + toNano(1),
      {
        destination: destDomain,
        recipient: provider.sender().address!.hash,
        amount: sendAmount,
      },
    );
  }
  console.log('DONE');
}
