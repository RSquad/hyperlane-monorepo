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

  let routers = await route.tokenRouter.getRouters();
  console.log('routers:');
  for (const key of routers.keys()) {
    const value = routers.get(key);
    if (value) {
      const tonAddress = new Address(0, value);
      console.log(`Domain: ${key}, Address: ${tonAddress.toString()}`);
    }
  }

  const dest_route = loadWarpRoute(provider, destDomain);

  let dest_routers = await dest_route.tokenRouter.getRouters();
  console.log('dest_routers:');
  for (const key of dest_routers.keys()) {
    const value = dest_routers.get(key);
    if (value) {
      const tonAddress = new Address(0, value);
      console.log(`Domain: ${key}, Address: ${tonAddress.toString()}`);
    }
  }

  return;
}
