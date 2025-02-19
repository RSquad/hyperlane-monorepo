import { compile } from '@ton/blueprint';
import {
  Cell,
  Dictionary,
  TransactionDescriptionGeneric,
  beginCell,
  toNano,
} from '@ton/core';
import {
  Blockchain,
  SandboxContract,
  SendMessageResult,
  TreasuryContract,
} from '@ton/sandbox';
import '@ton/test-utils';
import { FlatTransactionComparable } from '@ton/test-utils';

import { InterchainGasPaymaster } from '../wrappers/InterchainGasPaymaster';
import {
  JettonMinterContract,
  buildTokenMetadataCell,
} from '../wrappers/JettonMinter';
import { JettonWalletContract } from '../wrappers/JettonWallet';
import { MAILBOX_VERSION, Mailbox } from '../wrappers/Mailbox';
import { MerkleHookMock } from '../wrappers/MerkleHookMock';
import { MockIsm } from '../wrappers/MockIsm';
import { RecipientMock } from '../wrappers/RecipientMock';
import { TokenRouter } from '../wrappers/TokenRouter';
import {
  buildHookMetadataCell,
  buildMessageCell,
  buildMetadataCell,
} from '../wrappers/utils/builders';
import {
  METADATA_VARIANT,
  OpCodes,
  ProcessOpCodes,
} from '../wrappers/utils/constants';
import {
  TMailboxContractConfig,
  TMessage,
  TMultisigMetadata,
} from '../wrappers/utils/types';

import { expectTransactionFlow } from './utils/expect';
import { makeRandomBigint } from './utils/generators';

const buildTokenMessage = (tokenRecipient: Buffer, tokenAmount: bigint) => {
  return beginCell()
    .storeBuffer(tokenRecipient)
    .storeUint(tokenAmount, 256)
    .endCell();
};

const buildMessage = (
  origin: number,
  sender: Buffer,
  destination: number,
  recipient: Buffer,
  body: Cell,
  version: number = Mailbox.version,
): TMessage => {
  return {
    version,
    nonce: 0,
    origin,
    sender,
    destination,
    recipient,
    body,
  };
};

describe('TokenRouter', () => {
  let hypJettonCode: Cell;
  let mailboxCode: Cell;
  let requiredHookCode: Cell;
  let defaultHookCode: Cell;
  let mockIsmCode: Cell;
  let recipientCode: Cell;
  let minterCode: Cell;
  let walletCode: Cell;
  const burnAmount = 1000n;
  const destinationChain = 1234;
  const originChain = 4321;

  beforeAll(async () => {
    hypJettonCode = await compile('HypJetton');
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
  let originRouterMock: SandboxContract<TreasuryContract>;
  let destRouterMock: SandboxContract<TreasuryContract>;
  let tokenRouter: SandboxContract<TokenRouter>;
  let mailbox: SandboxContract<Mailbox>;
  let recipient: SandboxContract<RecipientMock>;
  let jettonMinter: SandboxContract<JettonMinterContract>;
  let jettonWallet: SandboxContract<JettonWalletContract>;
  let initialRequiredHook: SandboxContract<InterchainGasPaymaster>;
  let initialDefaultHook: SandboxContract<MerkleHookMock>;
  let initialDefaultIsm: SandboxContract<MockIsm>;
  let routers: Dictionary<number, Buffer>;
  const intialGasConfig = {
    gasOracle: makeRandomBigint(),
    gasOverhead: 0n,
    exchangeRate: 5n,
    gasPrice: 1000000000n,
  };
  const defaultHookConfig = {
    index: 0,
  };

  beforeEach(async () => {
    blockchain = await Blockchain.create();
    deployer = await blockchain.treasury('deployer');
    originRouterMock = await blockchain.treasury('originRouterMock');
    destRouterMock = await blockchain.treasury('destRouterMock');
    routers = Dictionary.empty(
      Dictionary.Keys.Uint(32),
      Dictionary.Values.Buffer(32),
    );
    routers.set(destinationChain, destRouterMock.address.hash);
    routers.set(originChain, originRouterMock.address.hash);

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
      version: Mailbox.version,
      localDomain: destinationChain,
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

    tokenRouter = blockchain.openContract(
      TokenRouter.createFromConfig(
        {
          ownerAddress: deployer.address,
          mailboxAddress: mailbox.address,
          jettonAddress: jettonMinter.address,
          routers,
        },
        hypJettonCode,
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

    const deployResult = await tokenRouter.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );

    expect(deployResult.transactions).toHaveTransaction({
      from: deployer.address,
      to: tokenRouter.address,
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

    await jettonMinter.sendMint(deployer.getSender(), {
      toAddress: deployer.address,
      responseAddress: deployer.address,
      jettonAmount: burnAmount,
      queryId: 0,
      value: toNano(0.1),
    });

    await jettonMinter.sendUpdateAdmin(deployer.getSender(), {
      value: toNano(0.1),
      newAdminAddress: tokenRouter.address,
    });

    expect((await jettonMinter.getAdmin())?.toString()).toStrictEqual(
      tokenRouter.address.toString(),
    );

    //await blockchain.setVerbosityForAddress(mailbox.address,'vm_logs_full');
  });

  it('process -> handle', async () => {
    const { amount: balanceBefore } = await jettonWallet.getBalance();
    const mintedAmount = 1000n;
    const hyperlaneMessage = buildMessage(
      originChain,
      originRouterMock.address.hash,
      destinationChain,
      tokenRouter.address.hash,
      buildTokenMessage(deployer.address.hash, mintedAmount),
    );
    const metadata: TMultisigMetadata = {
      originMerkleHook: Buffer.alloc(32),
      root: Buffer.alloc(32),
      index: 0n,
      signatures: [{ r: 0n, s: 0n, v: 0n }],
    };
    const res = await mailbox.sendProcess(deployer.getSender(), toNano('0.1'), {
      metadata,
      message: hyperlaneMessage,
    });

    expectTransactionFlow(res, [
      {
        from: deployer.address,
        to: mailbox.address,
        success: true,
        op: OpCodes.PROCESS,
      },
      {
        from: mailbox.address,
        to: tokenRouter.address,
        success: true,
        op: OpCodes.GET_ISM,
      },
      {
        from: tokenRouter.address,
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
        to: tokenRouter.address,
        success: true,
        op: OpCodes.HANDLE,
      },
      {
        from: tokenRouter.address,
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
    ]);

    const { amount: balanceAfter } = await jettonWallet.getBalance();
    expect(balanceAfter - balanceBefore).toBe(mintedAmount);
  });

  it('burn -> dispatch', async () => {
    const jettonAmount = 10n;
    const res = await jettonWallet.sendBurn(deployer.getSender(), {
      value: toNano(0.1),
      queryId: 0,
      jettonAmount: jettonAmount,
      responseAddress: deployer.address,
      destDomain: destinationChain,
      recipientAddr: tokenRouter.address.hash,
      message: beginCell()
        .storeBuffer(recipient.address.hash)
        .storeUint(jettonAmount, 256)
        .endCell(),
      hookMetadata: {
        variant: METADATA_VARIANT.STANDARD,
        msgValue: toNano('1'),
        gasLimit: 100000000n,
        refundAddress: deployer.address,
      },
    });
    expectTransactionFlow(res, [
      {
        from: deployer.address,
        to: jettonWallet.address,
        success: true,
        op: OpCodes.JETTON_BURN,
      },
      {
        from: jettonWallet.address,
        to: jettonMinter.address,
        success: true,
        op: OpCodes.JETTON_BURN_NOTIFICATION,
      },
      {
        from: jettonMinter.address,
        to: tokenRouter.address,
        success: true,
        op: OpCodes.JETTON_BURN_NOTIFICATION,
      },
      {
        from: tokenRouter.address,
        to: mailbox.address,
        success: true,
        op: OpCodes.DISPATCH,
      },
      {
        from: mailbox.address,
        to: initialRequiredHook.address,
        success: true,
        op: OpCodes.POST_DISPATCH,
      },
    ]);
  });

  it.skip('native send -> dispatch', async () => {
    const amount = toNano(2);
    const executionFee = toNano(1);

    const res = await tokenRouter.sendTransferRemote(
      deployer.getSender(),
      amount + executionFee,
      {
        destination: destinationChain,
        recipient: deployer.address.hash,
        amount,
      },
    );

    const tx = res.transactions.find(
      (tx) =>
        tx.address.toString(16) === tokenRouter.address.hash.toString('hex'),
    );
    expect(tx).toBeDefined();
    const descr = tx!.description as TransactionDescriptionGeneric;
    const fwdFees = descr.actionPhase!.totalFwdFees!;
    const actionFees = descr.actionPhase!.totalActionFees!;
    expectTransactionFlow(res, [
      {
        from: deployer.address,
        to: tokenRouter.address,
        success: true,
        op: OpCodes.TRANSFER_REMOTE,
        value: amount + executionFee,
        body: beginCell()
          .storeUint(OpCodes.TRANSFER_REMOTE, 32)
          .storeUint(0, 64)
          .storeUint(destinationChain, 32)
          .storeBuffer(deployer.address.hash, 32)
          .storeUint(amount, 256)
          .storeMaybeRef(null)
          .storeMaybeRef(null)
          .endCell(),
      },
      {
        from: tokenRouter.address,
        to: mailbox.address,
        success: true,
        op: OpCodes.DISPATCH,
        value: executionFee - tx!.totalFees.coins - fwdFees + actionFees,
        body: beginCell()
          .storeUint(OpCodes.DISPATCH, 32)
          .storeUint(0, 64)
          .storeUint(OpCodes.DISPATCH_INIT, 32)
          .storeUint(destinationChain, 32)
          .storeBuffer(routers.get(destinationChain)!, 32)
          .storeRef(
            beginCell()
              .storeBuffer(deployer.address.hash)
              .storeUint(amount, 256)
              .endCell(),
          )
          .storeRef(
            buildHookMetadataCell({
              variant: METADATA_VARIANT.STANDARD,
              msgValue: 0n,
              gasLimit: 0n,
              refundAddress: deployer.address,
            }),
          )
          .endCell(),
      },
    ]);
  });
});
