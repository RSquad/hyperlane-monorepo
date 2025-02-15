import { compile } from '@ton/blueprint';
import { Cell, beginCell, toNano } from '@ton/core';
import { Blockchain, SandboxContract, TreasuryContract } from '@ton/sandbox';
import '@ton/test-utils';

import { ProtocolFeeHook } from '../wrappers/ProtocolFeeHook';
import { Errors } from '../wrappers/utils/constants';

describe('ProtocolFeeHook', () => {
  let code: Cell;
  const maxProtocolFee = toNano('0.2');

  beforeAll(async () => {
    code = await compile('ProtocolFeeHook');
  });

  let blockchain: Blockchain;
  let deployer: SandboxContract<TreasuryContract>;
  let owner: SandboxContract<TreasuryContract>;
  let protocolFeeHook: SandboxContract<ProtocolFeeHook>;

  beforeEach(async () => {
    blockchain = await Blockchain.create();

    deployer = await blockchain.treasury('deployer');
    owner = await blockchain.treasury('owner');

    protocolFeeHook = blockchain.openContract(
      ProtocolFeeHook.createFromConfig(
        {
          hookType: 0,
          protocolFee: 0n,
          maxProtocolFee,
          beneficiary: deployer.address,
          owner: deployer.address,
        },
        code,
      ),
    );

    const deployResult = await protocolFeeHook.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );

    expect(deployResult.transactions).toHaveTransaction({
      from: deployer.address,
      to: protocolFeeHook.address,
      deploy: true,
      success: true,
    });
  });

  it('should deploy', async () => {
    // the check is done inside beforeEach
    // blockchain and protocolFeeHook are ready to use
  });

  it('should send post dispatch', async () => {
    const result = await protocolFeeHook.sendPostDispatch(
      deployer.getSender(),
      toNano('0.01'),
      {
        message: {
          version: 1,
          nonce: 2,
          origin: 0,
          sender: Buffer.alloc(32),
          destinationDomain: 0,
          recipient: Buffer.alloc(32),
          body: beginCell().storeUint(123, 32).endCell(),
        },
        hookMetadata: {
          variant: 0,
          msgValue: toNano('0.1'),
          gasLimit: 50000n,
          refundAddress: deployer.address,
        },
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: deployer.address,
      to: protocolFeeHook.address,
      success: true,
    });
  });

  it('should set protocol fee', async () => {
    const result = await protocolFeeHook.sendSetProtocolFee(
      deployer.getSender(),
      toNano('0.01'),
      {
        protocolFee: 100n,
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: deployer.address,
      to: protocolFeeHook.address,
      success: true,
    });

    const fee = await protocolFeeHook.getProtocolFee();
    expect(fee).toStrictEqual(100n);
  });

  it('should transfer ownership', async () => {
    const result = await protocolFeeHook.sendTransferOwnership(
      deployer.getSender(),
      toNano('0.01'),
      {
        ownerAddr: owner.address,
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: deployer.address,
      to: protocolFeeHook.address,
      success: true,
    });
  });

  it('should throw if msg value is too low', async () => {
    await protocolFeeHook.sendSetProtocolFee(
      deployer.getSender(),
      toNano('0.01'),
      {
        protocolFee: toNano('0.02'),
      },
    );

    const result = await protocolFeeHook.sendPostDispatch(
      deployer.getSender(),
      toNano('0.01'),
      {
        message: {
          version: 1,
          nonce: 2,
          origin: 0,
          sender: Buffer.alloc(32),
          destinationDomain: 0,
          recipient: Buffer.alloc(32),
          body: beginCell().storeUint(123, 32).endCell(),
        },
        hookMetadata: {
          variant: 0,
          msgValue: toNano('0.01'),
          gasLimit: 50000n,
          refundAddress: deployer.address,
        },
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: deployer.address,
      to: protocolFeeHook.address,
      success: false,
      exitCode: Errors.INSUFFICIENT_PROTOCOL_FEE,
    });
  });

  it('should throw if not owner on transfer ownership', async () => {
    const result = await protocolFeeHook.sendTransferOwnership(
      owner.getSender(),
      toNano('0.01'),
      {
        ownerAddr: owner.address,
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: owner.address,
      to: protocolFeeHook.address,
      success: false,
      exitCode: Errors.UNAUTHORIZED_SENDER,
    });
  });

  it('should throw if not owner on set beneficiary', async () => {
    const result = await protocolFeeHook.sendSetBeneficiary(
      owner.getSender(),
      toNano('0.01'),
      {
        beneficiaryAddr: owner.address,
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: owner.address,
      to: protocolFeeHook.address,
      success: false,
      exitCode: Errors.UNAUTHORIZED_SENDER,
    });
  });

  it('should throw if protocol fee too high', async () => {
    const result = await protocolFeeHook.sendSetProtocolFee(
      deployer.getSender(),
      toNano('0.01'),
      {
        protocolFee: maxProtocolFee + 1n,
      },
    );

    expect(result.transactions).toHaveTransaction({
      from: deployer.address,
      to: protocolFeeHook.address,
      success: false,
      exitCode: Errors.EXCEEDS_MAX_PROTOCOL_FEE,
    });
  });

  it('should get beneficiary', async () => {
    const beneficiary = await protocolFeeHook.getBeneficiary();
    expect(beneficiary.toString()).toStrictEqual(deployer.address.toString());
  });

  it('should get max protocol fee', async () => {
    const fee = await protocolFeeHook.getMaxProtocolFee();
    expect(fee).toStrictEqual(maxProtocolFee);
  });
});
