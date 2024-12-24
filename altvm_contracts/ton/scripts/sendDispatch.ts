import { NetworkProvider } from '@ton/blueprint';
import { Address, beginCell, toNano } from '@ton/core';
import { ethers } from 'ethers';

import * as deployedContracts from '../deployedContracts.json';
import { Mailbox } from '../wrappers/Mailbox';
import { THookMetadata } from '../wrappers/utils/types';

export async function run(provider: NetworkProvider) {
  const mailbox = provider.open(
    Mailbox.createFromAddress(Address.parse(deployedContracts.mailboxAddress)),
  );

  const wallet = ethers.Wallet.createRandom();

  const destAddr = Buffer.from(
    wallet.address.slice(2).padStart(64, '0'),
    'hex',
  );
  const destAddrTon = Address.parse(deployedContracts.recipientAddress).hash;

  const hookMetadata: THookMetadata = {
    variant: 0,
    msgValue: 1000n,
    gasLimit: 50000n,
    refundAddress: Address.parse(process.env.TON_ADDRESS!),
  };

  await mailbox.sendDispatch(provider.sender(), toNano('0.2'), {
    destDomain: 777001,
    recipientAddr: destAddrTon,
    message: beginCell().storeUint(123, 32).endCell(),
    hookMetadata,
    requiredValue: toNano('0.1'),
  });
}
