import { NetworkProvider, compile } from '@ton/blueprint';
import { OpenedContract, toNano } from '@ton/core';

import {
  JettonMinterContract,
  buildTokenMetadataCell,
} from '../wrappers/JettonMinter';
import { TokenCollateral } from '../wrappers/TokenCollateral';

async function deploy<T>(
  c: any,
  config: any,
  code: string,
  provider: NetworkProvider,
): Promise<OpenedContract<T>> {
  const contract = provider.open(
    c.createFromConfig(config, await compile(code)),
  );
  await contract.sendDeploy(provider.sender(), toNano('0.1'));
  await provider.waitForDeploy(contract.address);
  return contract;
}

export async function run(provider: NetworkProvider) {
  const ui = provider.ui();
  const mailboxAddress = await ui.inputAddress('Enter mailbox address:');
  const jettonParams = {
    name: 'TEST WRAPPED TON ' + Math.floor(Math.random() * 10000000),
    symbol: 'synTON',
    decimals: '9',
    description: 'test wrapped ton:' + Math.floor(Math.random() * 10000000),
  };

  const minter = await deploy<JettonMinterContract>(
    JettonMinterContract,
    {
      adminAddress: provider.sender().address,
      content: buildTokenMetadataCell(jettonParams),
      jettonWalletCode: await compile('JettonWallet'),
    },
    'JettonMinter',
    provider,
  );

  const tokenRouter = await deploy<TokenCollateral>(
    TokenCollateral,
    {
      jettonAddress: minter.address,
      mailboxAddress,
    },
    'TokenCollateral',
    provider,
  );

  await minter.sendUpdateAdmin(provider.sender(), {
    value: toNano(0.03),
    newAdminAddress: tokenRouter.address,
  });
  await provider.sender().send({ value: toNano(1), to: minter.address });
  console.log('JettonMinter deployed at', minter.address.toString());
  console.log('TokenRouter deployed at', tokenRouter.address.toString());
}
