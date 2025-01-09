import { NetworkProvider } from '@ton/blueprint';
import { Address, beginCell, Cell, Dictionary, toNano } from '@ton/core';
import { ValidatorAnnounce } from '../wrappers/ValidatorAnnounce';
import { buildValidators } from '../wrappers/utils/builders';
import { ethers } from 'ethers';

export async function run(provider: NetworkProvider) {
    // const validatorAnnounce = provider.open(ValidatorAnnounce.createFromAddress(Address.parse('EQD1y78zFUKPobC07ddM2Xs2iO1ihpyKybVh0HH-JsWQYCrl')));
    const validatorAnnounce = provider.open(ValidatorAnnounce.createFromAddress(Address.parse('EQAvcktAoqx6DwAdQLVuL6wmwUQPQK9MdIyuq4TzXkKNdCYH')));
    const sampleWallet = new ethers.Wallet(process.env.ETH_WALLET_PUBKEY!);

    const storageLocations = await validatorAnnounce.getAnnouncedStorageLocations(
        buildValidators({
            builder: beginCell(),
            validators: [BigInt(sampleWallet.address)],
        }).builder.endCell(),
    );


    console.log(storageLocations);
}
