import { NetworkProvider, compile } from '@ton/blueprint';
import { Address, toNano } from '@ton/core';

import { TokenCollateral } from '../wrappers/TokenCollateral';

export async function run(provider: NetworkProvider) {
  const tokenCollateral = provider.open(
    TokenCollateral.createFromConfig(
      {
        jettonAddress: Address.parse(''),
        mailboxAddress: Address.parse(''),
      },
      await compile('TokenCollateral'),
    ),
  );

  await tokenCollateral.sendDeploy(provider.sender(), toNano('0.1'));
  await provider.waitForDeploy(tokenCollateral.address);
}
