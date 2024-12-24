import { toNano } from '@ton/core';
import { MerkleTreeHook } from '../wrappers/MerkleTreeHook';
import { compile, NetworkProvider } from '@ton/blueprint';
import * as deployedContracts from '../deployedContracts.json';
import * as fs from 'fs';

export async function run(provider: NetworkProvider) {
    const merkleTreeHook = provider.open(MerkleTreeHook.createFromConfig({
        index: 0,
    }, await compile('MerkleTreeHook')));

    await merkleTreeHook.sendDeploy(provider.sender(), toNano('0.05'));

    await provider.waitForDeploy(merkleTreeHook.address);

    const data = {
        mailboxAddress: deployedContracts.mailboxAddress,
        interchainGasPaymasterAddress: deployedContracts.interchainGasPaymasterAddress,
        recipientAddress: deployedContracts.recipientAddress,
        multisigIsmAddress: deployedContracts.multisigIsmAddress,
        validatorAnnounceAddress: deployedContracts.validatorAnnounceAddress,
        merkleTreeHookAddress: merkleTreeHook.address.toString(),
    };

    fs.writeFileSync('./deployedContracts.json', JSON.stringify(data));
}
