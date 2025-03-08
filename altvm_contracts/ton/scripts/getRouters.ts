import { NetworkProvider } from '@ton/blueprint';
import { Address } from '@ton/core';
import * as fs from 'fs';
import * as path from 'path';

import { JettonMinterContract } from '../wrappers/JettonMinter';
import { TokenRouter } from '../wrappers/TokenRouter';

import { loadWarpRoute } from './common';
import { Route } from './types';

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
