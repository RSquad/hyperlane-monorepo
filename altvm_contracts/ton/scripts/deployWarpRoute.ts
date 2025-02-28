import { NetworkProvider, compile, sleep } from '@ton/blueprint';
import { Address, Dictionary, OpenedContract, toNano } from '@ton/core';
import * as fs from 'fs';
import * as path from 'path';

import {
  JettonMinterContract,
  buildTokenMetadataCell,
} from '../wrappers/JettonMinter';
import { TokenRouter } from '../wrappers/TokenRouter';

import { Route, TokenStandard } from './types';

async function deploy<T>(
  c: any,
  config: any,
  code: string,
  provider: NetworkProvider,
): Promise<OpenedContract<T>> {
  const codeCell = await compile(code);
  console.log('code hash:', codeCell.hash().toString('hex'));
  const contract = provider.open(c.createFromConfig(config, codeCell));
  await contract.sendDeploy(provider.sender(), toNano('0.1'));
  await provider.waitForDeploy(contract.address, 20, 3000);
  return contract;
}

async function deployWarpRoute(
  provider: NetworkProvider,
  tokenStandard: TokenStandard,
  mailboxAddress: Address,
): Promise<Route> {
  console.log('DEPLOY WARP ROUTE', tokenStandard);
  const params: Partial<Route> = {};
  let routerType = 'HypNative';
  const routers: Dictionary<number, Buffer> = Dictionary.empty(
    Dictionary.Keys.Uint(32),
    Dictionary.Values.Buffer(32),
  );
  const jettonParams = {
    name: 'Synthetic TON ' + Math.floor(Math.random() * 10000000),
    symbol: 'TsynTON',
    decimals: '9',
    description: 'test synthetic ton',
  };

  if (tokenStandard === TokenStandard.Native) {
    routerType = 'HypNative';
  } else if (
    tokenStandard === TokenStandard.Synthetic ||
    tokenStandard === TokenStandard.Collateral
  ) {
    console.log('Deploy jetton');
    params.jettonMinter = await deploy<JettonMinterContract>(
      JettonMinterContract,
      {
        adminAddress: provider.sender().address,
        content: buildTokenMetadataCell(jettonParams),
        jettonWalletCode: await compile('JettonWallet'),
      },
      'JettonMinter',
      provider,
    );

    routerType =
      tokenStandard === TokenStandard.Synthetic
        ? 'HypJetton'
        : 'HypJettonCollateral';
  }
  console.log('Deploy router with jetton', params.jettonMinter?.address);
  params.tokenRouter = await deploy<TokenRouter>(
    TokenRouter,
    {
      ownerAddress: provider.sender().address,
      jettonAddress: params.jettonMinter?.address,
      mailboxAddress,
      routers,
      JettonWalletCode: params.jettonMinter
        ? await compile('JettonWallet')
        : undefined,
    },
    routerType,
    provider,
  );
  if (params.jettonMinter) {
    await params.jettonMinter.sendUpdateAdmin(provider.sender(), {
      value: toNano(0.03),
      newAdminAddress: params.tokenRouter.address,
    });
    await provider
      .sender()
      .send({ value: toNano(1), to: params.jettonMinter.address });
  }

  return {
    jettonMinter: params.jettonMinter,
    tokenRouter: params.tokenRouter!,
  };
}

function writeWarpRoute(domain: number, route: Route) {
  const filePath = path.join(__dirname, `../warp-contracts-${domain}.json`);
  fs.writeFileSync(
    filePath,
    JSON.stringify(
      {
        jetton: route.jettonMinter?.address.toString(),
        router: route.tokenRouter.address.toString(),
      },
      null,
      ' ',
    ),
  );
}

export async function run(provider: NetworkProvider) {
  const originDomain = Number(process.env.ORIGIN_DOMAIN);
  const destDomain = Number(process.env.DESTINATION_DOMAIN);
  const origTokenStandard =
    (process.env.ORIGIN_TOKEN_STANDARD as TokenStandard) ??
    TokenStandard.Native;
  const destTokenStandard =
    (process.env.DESTINATION_TOKEN_STANDARD as TokenStandard) ??
    TokenStandard.Synthetic;
  const origMailboxAddress = Address.parse(process.env.ORIGIN_MAILBOX!);
  const destMailboxAddress = Address.parse(process.env.DESTINATION_MAILBOX!);

  const ui = provider.ui();

  const warp1 = await deployWarpRoute(
    provider,
    origTokenStandard,
    origMailboxAddress,
  );

  const warp2 = await deployWarpRoute(
    provider,
    destTokenStandard,
    destMailboxAddress,
  );

  console.log('Set destination router');
  while (true) {
    await warp1.tokenRouter.sendSetRouter(provider.sender(), toNano(0.03), {
      domain: destDomain,
      router: warp2.tokenRouter.address.hash,
    });
    const routers = await warp1.tokenRouter.getRouters();
    if (routers.get(destDomain)) break;
    await sleep(5000);
  }
  console.log('Done');
  console.log('Set origin router');
  while (true) {
    await warp2.tokenRouter.sendSetRouter(provider.sender(), toNano(0.03), {
      domain: originDomain,
      router: warp1.tokenRouter.address.hash,
    });
    const routers = await warp2.tokenRouter.getRouters();
    if (routers.get(originDomain)) break;
    await sleep(5000);
  }
  console.log('Done');
  console.log(
    `Warp route ${originDomain} (${origTokenStandard}) -> ${destDomain} (${destTokenStandard}):`,
  );

  console.log(
    originDomain,
    ' JettonMinter:',
    warp1.jettonMinter?.address.toString(),
  );
  console.log(
    originDomain,
    ' TokenRouter :',
    warp1.tokenRouter.address.toString(),
  );
  console.log(
    destDomain,
    ' JettonMinter:',
    warp2.jettonMinter?.address.toString(),
  );
  console.log(
    destDomain,
    ' TokenRouter :',
    warp2.tokenRouter.address.toString(),
  );

  writeWarpRoute(originDomain, warp1);
  writeWarpRoute(destDomain, warp2);
}
