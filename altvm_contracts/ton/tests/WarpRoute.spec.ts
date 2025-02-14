import { compile } from '@ton/blueprint';
import { Cell, Dictionary, beginCell, toNano } from '@ton/core';
import {
  Blockchain,
  SandboxContract,
  SendMessageResult,
  TreasuryContract,
} from '@ton/sandbox';
import '@ton/test-utils';

import { InterchainGasPaymaster } from '../wrappers/InterchainGasPaymaster';
import {
  JettonMinterContract,
  buildTokenMetadataCell,
} from '../wrappers/JettonMinter';
import { JettonWalletContract } from '../wrappers/JettonWallet';
import { Mailbox } from '../wrappers/Mailbox';
import { MerkleHookMock } from '../wrappers/MerkleHookMock';
import { MockIsm } from '../wrappers/MockIsm';
import { RecipientMock } from '../wrappers/RecipientMock';
import { TokenCollateral } from '../wrappers/TokenCollateral';
import { OpCodes, ProcessOpCodes } from '../wrappers/utils/constants';
import {
  TMailboxContractConfig,
  TMultisigMetadata,
} from '../wrappers/utils/types';

import { makeRandomBigint } from './utils/generators';

const buildTokenMessage = (
  collateralAddr: Buffer,
  recipient: Buffer,
  amount: bigint,
  sender: Buffer,
  version: number = 1,
  destinationDomain: number = 0,
) => {
  return {
    version,
    nonce: 0,
    origin: 0,
    sender: sender,
    destinationDomain,
    recipient: collateralAddr,
    body: beginCell().storeBuffer(recipient).storeUint(amount, 128).endCell(),
  };
};

describe('TokenCollateral', () => {
  let code: Cell;
  let mailboxCode: Cell;
  let requiredHookCode: Cell;
  let defaultHookCode: Cell;
  let mockIsmCode: Cell;
  let recipientCode: Cell;
  let minterCode: Cell;
  let walletCode: Cell;

  beforeAll(async () => {
    code = await compile('TokenCollateral');
    mailboxCode = await compile('Mailbox');
    requiredHookCode = await compile('InterchainGasPaymaster');
    defaultHookCode = await compile('MerkleHookMock');
    mockIsmCode = await compile('MockIsm');
    recipientCode = await compile('RecipientMock');
    minterCode = await compile('JettonMinter');
    walletCode = await compile('JettonWallet');
  });

  let blockchain: Blockchain;
  let deployer: SandboxContract<TreasuryContract>;
  let tokenCollateral: SandboxContract<TokenCollateral>;
  let mailbox: SandboxContract<Mailbox>;
  let recipient: SandboxContract<RecipientMock>;
  let jettonMinter: SandboxContract<JettonMinterContract>;
  let jettonWallet: SandboxContract<JettonWalletContract>;
  let initialRequiredHook: SandboxContract<InterchainGasPaymaster>;
  let initialDefaultHook: SandboxContract<MerkleHookMock>;
  let initialDefaultIsm: SandboxContract<MockIsm>;

  beforeEach(async () => {
    blockchain = await Blockchain.create();
    deployer = await blockchain.treasury('deployer');

    const intialGasConfig = {
      gasOracle: makeRandomBigint(),
      gasOverhead: 0n,
      exchangeRate: 5n,
      gasPrice: 1000000000n,
    };

    const dictDestGasConfig = Dictionary.empty(
      InterchainGasPaymaster.GasConfigKey,
      InterchainGasPaymaster.GasConfigValue,
    );
    dictDestGasConfig.set(0, intialGasConfig);

    const requiredHookConfig = {
      owner: deployer.address,
      beneficiary: deployer.address,
      hookType: 0,
      hookMetadata: Cell.EMPTY,
      destGasConfig: dictDestGasConfig,
    };

    const defaultHookConfig = {
      index: 0,
    };

    initialRequiredHook = blockchain.openContract(
      InterchainGasPaymaster.createFromConfig(
        requiredHookConfig,
        requiredHookCode,
      ),
    );
    initialDefaultHook = blockchain.openContract(
      MerkleHookMock.createFromConfig(defaultHookConfig, defaultHookCode),
    );
    initialDefaultIsm = blockchain.openContract(
      MockIsm.createFromConfig({}, mockIsmCode),
    );
    recipient = blockchain.openContract(
      RecipientMock.createFromConfig(
        {
          ismAddr: initialDefaultIsm.address,
        },
        recipientCode,
      ),
    );

    const initConfig: TMailboxContractConfig = {
      version: 1,
      localDomain: 0,
      nonce: 0,
      latestDispatchedId: 0n,
      defaultIsm: initialDefaultIsm.address,
      defaultHookAddr: initialDefaultHook.address,
      requiredHookAddr: initialRequiredHook.address,
      owner: deployer.address,
      deliveries: Dictionary.empty(Mailbox.DeliveryKey, Mailbox.DeliveryValue),
    };

    mailbox = blockchain.openContract(
      Mailbox.createFromConfig(initConfig, mailboxCode),
    );

    const jettonParams = {
      name: 'test ' + Math.floor(Math.random() * 10000000),
      symbol: 'test',
      decimals: '8',
    };

    jettonMinter = blockchain.openContract(
      JettonMinterContract.createFromConfig(
        {
          adminAddress: deployer.address,
          content: buildTokenMetadataCell(jettonParams),
          jettonWalletCode: walletCode,
        },
        minterCode,
      ),
    );

    jettonWallet = blockchain.openContract(
      JettonWalletContract.createFromConfig(
        {
          ownerAddress: deployer.address,
          minterAddress: jettonMinter.address,
        },
        walletCode,
      ),
    );

    tokenCollateral = blockchain.openContract(
      TokenCollateral.createFromConfig(
        {
          mailboxAddress: mailbox.address,
          jettonAddress: jettonMinter.address,
        },
        code,
      ),
    );

    const deployMboxRes = await mailbox.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );
    const deployRecipientRes = await recipient.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );
    const deployIsmRes = await initialDefaultIsm.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );
    const deployIgpRes = await initialRequiredHook.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );
    const deployDefaultHookRes = await initialDefaultHook.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );

    expect(deployMboxRes.transactions).toHaveTransaction({
      from: deployer.address,
      to: mailbox.address,
      deploy: true,
      success: true,
    });

    expect(deployIgpRes.transactions).toHaveTransaction({
      from: deployer.address,
      to: initialRequiredHook.address,
      deploy: true,
      success: true,
    });

    expect(deployIsmRes.transactions).toHaveTransaction({
      from: deployer.address,
      to: initialDefaultIsm.address,
      deploy: true,
      success: true,
    });

    expect(deployDefaultHookRes.transactions).toHaveTransaction({
      from: deployer.address,
      to: initialDefaultHook.address,
      deploy: true,
      success: true,
    });

    expect(deployRecipientRes.transactions).toHaveTransaction({
      from: deployer.address,
      to: recipient.address,
      deploy: true,
      success: true,
    });

    const deployResult = await tokenCollateral.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );

    expect(deployResult.transactions).toHaveTransaction({
      from: deployer.address,
      to: tokenCollateral.address,
      deploy: true,
      success: true,
    });

    const deployMinterRes = await jettonMinter.sendDeploy(
      deployer.getSender(),
      toNano('1.5'),
    );

    expect(deployMinterRes.transactions).toHaveTransaction({
      from: deployer.address,
      to: jettonMinter.address,
      deploy: true,
      success: true,
    });

    await jettonMinter.sendUpdateAdmin(deployer.getSender(), {
      value: toNano('0.1'),
      newAdminAddress: tokenCollateral.address,
    });

    expect((await jettonMinter.getAdmin())?.toString()).toStrictEqual(
      tokenCollateral.address.toString(),
    );

    //await blockchain.setVerbosityForAddress(mailbox.address,'vm_logs_full');
  });

  const expectWarpRouteSucceeded = (res: SendMessageResult) => {
    const expectedTransactions = [
      {
        from: deployer.address,
        to: mailbox.address,
        success: true,
        op: OpCodes.PROCESS,
      },
      {
        from: mailbox.address,
        to: tokenCollateral.address,
        success: true,
        op: OpCodes.GET_ISM,
      },
      {
        from: tokenCollateral.address,
        to: mailbox.address,
        success: true,
        op: OpCodes.PROCESS,
        body: (x: Cell | undefined): boolean => {
          if (!x) return false;
          const s = x!.beginParse();
          s.skip(32 + 64);
          return s.loadUint(32) == ProcessOpCodes.VERIFY;
        },
      },
      {
        from: mailbox.address,
        to: initialDefaultIsm.address,
        success: true,
        op: OpCodes.VERIFY,
      },
      {
        from: initialDefaultIsm.address,
        to: mailbox.address,
        success: true,
        op: OpCodes.PROCESS,
        body: (x: Cell | undefined): boolean => {
          if (!x) return false;
          const s = x!.beginParse();
          s.skip(32 + 64);
          return s.loadUint(32) == ProcessOpCodes.DELIVER_MESSAGE;
        },
      },
      {
        from: mailbox.address,
        to: tokenCollateral.address,
        success: true,
        op: OpCodes.HANDLE,
      },
      {
        from: tokenCollateral.address,
        to: jettonMinter.address,
        success: true,
        op: OpCodes.JETTON_MINT,
      },
      {
        from: jettonMinter.address,
        to: jettonWallet.address,
        success: true,
        op: OpCodes.JETTON_INTERNAL_TRANSFER,
      },
    ];

    expectedTransactions.forEach((ex, i) => {
      try {
        expect([res.transactions[i + 1]]).toHaveTransaction({
          ...ex,
        });
      } catch (err) {
        console.log('Failed exp:', i);
        throw err;
      }
    });
  };

  it.only('should receive tokens', async () => {
    const { amount: balanceBefore } = await jettonWallet.getBalance();
    const mintedAmount = 1000n;
    const hyperlaneMessage = buildTokenMessage(
      tokenCollateral.address.hash,
      deployer.address.hash,
      mintedAmount,
      Buffer.alloc(32),
    );
    const metadata: TMultisigMetadata = {
      originMerkleHook: Buffer.alloc(32),
      root: Buffer.alloc(32),
      index: 0n,
      signatures: [{ r: 0n, s: 0n, v: 0n }],
    };
    const res = await mailbox.sendProcess(deployer.getSender(), toNano('0.1'), {
      blockNumber: 0,
      metadata,
      message: hyperlaneMessage,
    });

    expectWarpRouteSucceeded(res);

    const { amount: balanceAfter } = await jettonWallet.getBalance();
    expect(balanceAfter - balanceBefore).toBe(mintedAmount);
  });

  it('should send tokens', async () => {
    const jettonAmount = 10n;
    const burnRes = await jettonWallet.sendBurn(deployer.getSender(), {
      value: toNano('0.1'),
      queryId: 0,
      jettonAmount: jettonAmount,
      responseAddress: deployer.address,
      destDomain: 0n,
      recipientAddr: tokenCollateral.address.hash,
      message: beginCell()
        .storeBuffer(recipient.address.hash)
        .storeCoins(jettonAmount)
        .endCell(),
      hookMetadata: {
        variant: 0,
        msgValue: toNano('1'),
        gasLimit: 100000000n,
        refundAddress: deployer.address,
      },
    });

    expect(burnRes.transactions).toHaveTransaction({
      from: jettonWallet.address,
      to: jettonMinter.address,
      success: true,
      op: OpCodes.JETTON_BURN,
    });

    expect(burnRes.transactions).toHaveTransaction({
      from: jettonMinter.address,
      to: tokenCollateral.address,
      success: true,
      op: OpCodes.JETTON_BURN_NOTIFICATION,
    });

    expect(burnRes.transactions).toHaveTransaction({
      from: tokenCollateral.address,
      to: mailbox.address,
      success: true,
      op: OpCodes.DISPATCH,
    });

    expect(burnRes.transactions).toHaveTransaction({
      from: mailbox.address,
      to: initialRequiredHook.address,
      success: true,
      op: OpCodes.POST_DISPATCH,
    });

    expect(burnRes.transactions).toHaveTransaction({
      from: mailbox.address,
      to: initialDefaultHook.address,
      success: true,
      op: OpCodes.POST_DISPATCH,
    });
  });
});
