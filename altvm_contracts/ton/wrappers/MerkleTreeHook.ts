import { Address, beginCell, Cell, Contract, contractAddress, ContractProvider, Dictionary, Sender, SendMode } from '@ton/core';
import { OpCodes } from './utils/constants';
import { buildHookMetadataCell } from './utils/builders';
import { THookMetadata } from './utils/types';

export type MerkleTreeHookConfig = {
    index: number;
};

export function merkleTreeHookConfigToCell(config: MerkleTreeHookConfig): Cell {
    return beginCell()
        .storeUint(config.index, 256)
        .storeDict(
            Dictionary.empty(
                Dictionary.Keys.BigUint(256),
                Dictionary.Values.BigUint(256)
            )
        ).endCell();
}

export class MerkleTreeHook implements Contract {
    constructor(readonly address: Address, readonly init?: { code: Cell; data: Cell }) {}

    static createFromAddress(address: Address) {
        return new MerkleTreeHook(address);
    }

    static createFromConfig(config: MerkleTreeHookConfig, code: Cell, workchain = 0) {
        const data = merkleTreeHookConfigToCell(config);
        const init = { code, data };
        return new MerkleTreeHook(contractAddress(workchain, init), init);
    }

    async sendDeploy(provider: ContractProvider, via: Sender, value: bigint) {
        await provider.internal(via, {
            value,
            sendMode: SendMode.PAY_GAS_SEPARATELY,
            body: beginCell().endCell(),
        });
    }

    async sendPostDispatch(
        provider: ContractProvider,
        via: Sender,
        value: bigint,
        opts: {
            messageId: bigint;
            destDomain: number;
            refundAddr: Address;
            hookMetadata: THookMetadata;
            queryId?: number;
        },
    ) {
        await provider.internal(via, {
            value,
            sendMode: SendMode.PAY_GAS_SEPARATELY,
            body: beginCell()
                .storeUint(OpCodes.POST_DISPATCH, 32)
                .storeUint(opts.queryId ?? 0, 64)
                .storeUint(opts.messageId, 256)
                .storeUint(opts.destDomain, 32)
                .storeAddress(opts.refundAddr)
                .storeRef(buildHookMetadataCell(opts.hookMetadata))
                .endCell(),
        });
    }

    async getRoot(provider: ContractProvider) {
        const result = await provider.get('get_root', []);
        return result.stack.readBigNumber();
    }

    async getCount(provider: ContractProvider) {
        const result = await provider.get('get_count', []);
        return result.stack.readNumber();
    }

    async getLatestCheckpoint(provider: ContractProvider) {
        const result = await provider.get('get_latest_checkpoint', []);
        const root = result.stack.readNumber();
        const index = result.stack.readNumber();
        return { root, index };
    }
}
