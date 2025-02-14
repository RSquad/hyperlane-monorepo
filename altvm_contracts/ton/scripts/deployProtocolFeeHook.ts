import { NetworkProvider, compile } from '@ton/blueprint';
import { toNano } from '@ton/core';

import { ProtocolFeeHook } from '../wrappers/ProtocolFeeHook';

export async function run(provider: NetworkProvider) {
  const protocolFeeHook = provider.open(
    ProtocolFeeHook.createFromConfig({}, await compile('ProtocolFeeHook')),
  );

  await protocolFeeHook.sendDeploy(provider.sender(), toNano('0.05'));

  await provider.waitForDeploy(protocolFeeHook.address);

  // run methods on `protocolFeeHook`
}
