import { NetworkProvider, compile } from '@ton/blueprint';
import { toNano } from '@ton/core';

import { TokenCollateral } from '../wrappers/TokenCollateral';

export async function run(provider: NetworkProvider) {
  const tokenCollateral = provider.open(
    TokenCollateral.createFromConfig({}, await compile('TokenCollateral')),
  );

  await tokenCollateral.sendDeploy(provider.sender(), toNano('0.05'));

  await provider.waitForDeploy(tokenCollateral.address);
}
