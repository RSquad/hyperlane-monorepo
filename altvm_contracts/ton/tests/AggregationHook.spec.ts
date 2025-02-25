import { compile } from '@ton/blueprint';
import { Cell, Dictionary, beginCell, toNano } from '@ton/core';
import { Blockchain, SandboxContract, TreasuryContract } from '@ton/sandbox';
import '@ton/test-utils';

import { AggregationHook } from '../wrappers/AggregationHook';
import { ProtocolFeeHook } from '../wrappers/ProtocolFeeHook';
import { OpCodes } from '../wrappers/utils/constants';

describe('AggregationHook', () => {
  let code: Cell;

  beforeAll(async () => {
    code = await compile('AggregationHook');
  });

  let blockchain: Blockchain;
  let deployer: SandboxContract<TreasuryContract>;
  let sampleHook: SandboxContract<TreasuryContract>;
  let mockHook: SandboxContract<TreasuryContract>;
  let aggregationHook: SandboxContract<AggregationHook>;
  let protocolFeeHook: SandboxContract<ProtocolFeeHook>;

  beforeEach(async () => {
    blockchain = await Blockchain.create();

    deployer = await blockchain.treasury('deployer');
    sampleHook = await blockchain.treasury('sampleHook');
    mockHook = await blockchain.treasury('mockHook');

    aggregationHook = blockchain.openContract(
      AggregationHook.createFromConfig({}, code),
    );

    protocolFeeHook = blockchain.openContract(
      ProtocolFeeHook.createFromConfig(
        {
          protocolFee: 0n,
          maxProtocolFee: 0n,
          beneficiary: deployer.address,
          owner: deployer.address,
        },
        code,
      ),
    );

    const deployResult = await aggregationHook.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );

    expect(deployResult.transactions).toHaveTransaction({
      from: deployer.address,
      to: aggregationHook.address,
      deploy: true,
      success: true,
    });
  });

  it('should post dispatch', async () => {
    const hooksArr = [
      sampleHook.address,
      mockHook.address,
      protocolFeeHook.address,
    ];
    const hooks = Dictionary.empty(
      Dictionary.Keys.Uint(32),
      Dictionary.Values.Address(),
    );
    hooksArr.forEach((addr, i) => {
      hooks.set(i, addr);
    });
    const res = await aggregationHook.sendPostDispatch(
      deployer.getSender(),
      toNano('0.1'),
      {
        message: {
          version: 1,
          nonce: 2,
          origin: 0,
          sender: Buffer.alloc(32),
          destination: 0,
          recipient: Buffer.alloc(32),
          body: beginCell().storeDict(hooks).endCell(),
        },
        hookMetadata: {
          variant: 0,
          msgValue: 0n,
          gasLimit: 0n,
          refundAddress: deployer.address,
        },
      },
    );

    expect(res.transactions).toHaveTransaction({
      from: deployer.address,
      to: aggregationHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });
    expect(res.transactions).toHaveTransaction({
      from: aggregationHook.address,
      to: sampleHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });
    expect(res.transactions).toHaveTransaction({
      from: aggregationHook.address,
      to: mockHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });
    expect(res.transactions).toHaveTransaction({
      from: aggregationHook.address,
      to: protocolFeeHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });
  });
});
