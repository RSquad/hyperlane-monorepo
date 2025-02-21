import { NetworkProvider } from '@ton/blueprint';
import { Address, OpenedContract, beginCell, toNano } from '@ton/core';
import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

import { JettonMinterContract } from '../wrappers/JettonMinter';
import { TokenRouter } from '../wrappers/TokenRouter';
import { THookMetadata } from '../wrappers/utils/types';

import { Route, TokenStandard } from './types';

function loadWarpRoute(provider: NetworkProvider, domain: number): Route {
  const filePath = path.join(__dirname, `../warp-contracts-${domain}.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Deployed contracts file not found: ${filePath}`);
  }
  const addrs = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  return {
    tokenRouter: provider.open(
      TokenRouter.createFromAddress(Address.parse(addrs.router)),
    ),
    jettonMinter: addrs.jetton
      ? provider.open(
          JettonMinterContract.createFromAddress(Address.parse(addrs.jetton)),
        )
      : undefined,
  };
}

export async function run(provider: NetworkProvider) {
  const originDomain = Number(process.env.ORIGIN_DOMAIN);
  const destDomain = Number(process.env.DESTINATION_DOMAIN);
  const origTokenStandard =
    (process.env.ORIGIN_TOKEN_STANDARD as TokenStandard) ??
    TokenStandard.Native;
  const sendAmount = toNano(process.env.AMOUNT!);

  const route = loadWarpRoute(provider, originDomain);
  console.log(`Dispatching from domain ${originDomain} to ${destDomain}`);

  if (origTokenStandard === TokenStandard.Native) {
    await route.tokenRouter.sendTransferRemote(
      provider.sender(),
      sendAmount + toNano(0.5),
      {
        destination: destDomain,
        recipient: provider.sender().address!.hash,
        amount: sendAmount,
      },
    );
  }
  console.log('DONE');
}
