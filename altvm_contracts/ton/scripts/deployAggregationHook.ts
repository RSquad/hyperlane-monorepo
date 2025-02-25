import { NetworkProvider, compile } from '@ton/blueprint';
import { toNano } from '@ton/core';

import { AggregationHook } from '../wrappers/AggregationHook';

export async function run(provider: NetworkProvider) {
  const aggregationHook = provider.open(
    AggregationHook.createFromConfig({}, await compile('AggregationHook')),
  );

  await aggregationHook.sendDeploy(provider.sender(), toNano('0.05'));

  await provider.waitForDeploy(aggregationHook.address);

  // run methods on `aggregationHook`
}
