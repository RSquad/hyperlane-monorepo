import { compile } from '@ton/blueprint';
import { Cell, Dictionary, beginCell, toNano } from '@ton/core';
import { Blockchain, SandboxContract, TreasuryContract } from '@ton/sandbox';
import '@ton/test-utils';

import { AggregationHook } from '../wrappers/AggregationHook';
import { InterchainGasPaymaster } from '../wrappers/InterchainGasPaymaster';
import { Mailbox } from '../wrappers/Mailbox';
import { MerkleHookMock } from '../wrappers/MerkleHookMock';
import { ProtocolFeeHook } from '../wrappers/ProtocolFeeHook';
import { OpCodes, answer } from '../wrappers/utils/constants';

import { makeRandomBigint } from './utils/generators';

const deployAndExpectSuccess = async (
  contract: SandboxContract<any>,
  deployer: SandboxContract<TreasuryContract>,
) => {
  const deployResult = await contract.sendDeploy(
    deployer.getSender(),
    toNano('0.05'),
  );

  expect(deployResult.transactions).toHaveTransaction({
    from: deployer.address,
    to: contract.address,
    deploy: true,
    success: true,
  });
};

describe('AggregationHook', () => {
  let code: Cell;
  let protocolFeeHookCode: Cell;
  let merkleHookCode: Cell;
  let igpHookCode: Cell;

  beforeAll(async () => {
    code = await compile('AggregationHook');
    protocolFeeHookCode = await compile('ProtocolFeeHook');
    merkleHookCode = await compile('MerkleHookMock');
    igpHookCode = await compile('InterchainGasPaymaster');
  });

  let blockchain: Blockchain;
  let deployer: SandboxContract<TreasuryContract>;
  let aggregationHook: SandboxContract<AggregationHook>;
  let protocolFeeHook: SandboxContract<ProtocolFeeHook>;
  let merkleHook: SandboxContract<MerkleHookMock>;
  let igpHook: SandboxContract<InterchainGasPaymaster>;

  beforeEach(async () => {
    blockchain = await Blockchain.create();

    deployer = await blockchain.treasury('deployer');

    merkleHook = blockchain.openContract(
      MerkleHookMock.createFromConfig(
        {
          index: 0,
        },
        merkleHookCode,
      ),
    );

    const intialGasConfig = {
      gasOracle: makeRandomBigint(),
      gasOverhead: 0n,
      exchangeRate: 1n,
      gasPrice: 1000000000n,
    };

    const dictDestGasConfig = Dictionary.empty(
      InterchainGasPaymaster.GasConfigKey,
      InterchainGasPaymaster.GasConfigValue,
    );
    dictDestGasConfig.set(0, intialGasConfig);

    const config = {
      owner: deployer.address,
      beneficiary: deployer.address,
      hookType: 0,
      hookMetadata: Cell.EMPTY,
      destGasConfig: dictDestGasConfig,
    };

    igpHook = blockchain.openContract(
      InterchainGasPaymaster.createFromConfig(config, igpHookCode),
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

    const hooksArr = [merkleHook.address, igpHook.address];
    const hooks = Dictionary.empty(
      Dictionary.Keys.Uint(8),
      Dictionary.Values.Address(),
    );
    hooksArr.forEach((addr, i) => {
      hooks.set(i, addr);
    });

    const curHookIndex = Dictionary.empty(
      Dictionary.Keys.Uint(64),
      Dictionary.Values.Cell(),
    );

    aggregationHook = blockchain.openContract(
      AggregationHook.createFromConfig(
        {
          mailboxAddr: deployer.address,
          hooks,
          curHookIndex,
        },
        code,
      ),
    );

    await deployAndExpectSuccess(aggregationHook, deployer);
    await deployAndExpectSuccess(protocolFeeHook, deployer);
    await deployAndExpectSuccess(merkleHook, deployer);
    await deployAndExpectSuccess(igpHook, deployer);
  });

  it('should post dispatch', async () => {
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
          body: beginCell().storeUint(123, 32).endCell(),
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
      to: merkleHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });

    expect(res.transactions).toHaveTransaction({
      from: merkleHook.address,
      to: aggregationHook.address,
      op: answer(OpCodes.POST_DISPATCH),
      success: true,
    });

    expect(res.transactions).toHaveTransaction({
      from: aggregationHook.address,
      to: igpHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });

    expect(res.transactions).toHaveTransaction({
      from: igpHook.address,
      to: aggregationHook.address,
      op: answer(OpCodes.POST_DISPATCH),
      success: true,
    });

    expect(res.transactions).toHaveTransaction({
      from: aggregationHook.address,
      to: deployer.address,
      op: answer(OpCodes.POST_DISPATCH),
      success: true,
    });
  });
});
