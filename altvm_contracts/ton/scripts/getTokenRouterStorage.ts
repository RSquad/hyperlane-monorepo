import { NetworkProvider } from '@ton/blueprint';
import { Address } from '@ton/core';
import * as fs from 'fs';
import * as path from 'path';

import { JettonMinterContract } from '../wrappers/JettonMinter';
import { TokenRouter } from '../wrappers/TokenRouter';

import { Route } from './types';

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

  const route = loadWarpRoute(provider, originDomain);

  let storage = await route.tokenRouter.getStorage();
  console.log(
    'TokenRouterConfig:',
    JSON.stringify(
      storage,
      (key, value) => {
        if (
          value &&
          typeof value.toString === 'function' &&
          value.constructor.name === 'Address'
        ) {
          return value.toString();
        }
        if (Buffer.isBuffer(value)) {
          return value.toString('hex');
        }
        return value;
      },
      2,
    ),
  );
}
