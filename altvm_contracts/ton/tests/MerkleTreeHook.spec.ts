import { compile } from '@ton/blueprint';
import { Cell, toNano } from '@ton/core';
import { Blockchain, SandboxContract, TreasuryContract } from '@ton/sandbox';
import '@ton/test-utils';

import { MerkleTreeHook } from '../wrappers/MerkleTreeHook';
import { OpCodes } from '../wrappers/utils/constants';

describe('MerkleTreeHook', () => {
  let code: Cell;

  beforeAll(async () => {
    code = await compile('MerkleTreeHook');
  });

  let blockchain: Blockchain;
  let deployer: SandboxContract<TreasuryContract>;
  let merkleTreeHook: SandboxContract<MerkleTreeHook>;

  beforeEach(async () => {
    blockchain = await Blockchain.create();

    merkleTreeHook = blockchain.openContract(
      MerkleTreeHook.createFromConfig(
        {
          index: 0,
        },
        code,
      ),
    );

    deployer = await blockchain.treasury('deployer');

    const deployResult = await merkleTreeHook.sendDeploy(
      deployer.getSender(),
      toNano('0.05'),
    );

    expect(deployResult.transactions).toHaveTransaction({
      from: deployer.address,
      to: merkleTreeHook.address,
      deploy: true,
      success: true,
    });
  });

  it('should post dispatch', async () => {
    const res = await merkleTreeHook.sendPostDispatch(
      deployer.getSender(),
      toNano('0.1'),
      {
        messageId: 1n,
        destDomain: 0,
        refundAddr: deployer.address,
        hookMetadata: {
          variant: 0,
          msgValue: toNano('0.1'),
          gasLimit: 50000n,
          refundAddress: deployer.address,
        },
      },
    );

    expect(res.transactions).toHaveTransaction({
      from: deployer.address,
      to: merkleTreeHook.address,
      op: OpCodes.POST_DISPATCH,
      success: true,
    });
    expect(res.externals).toHaveLength(1);
    const count = await merkleTreeHook.getCount();
    expect(count).toBeTruthy();
  });

  it('should return root', async () => {
    const res = await merkleTreeHook.getRoot();
    expect(res).toBeTruthy();
  });

  it('should return root and count', async () => {
    const res = await merkleTreeHook.getTree();
    console.log(res.tree);
    expect(res.tree).toBeTruthy();
    expect(res.count).toStrictEqual(0);
  });
});
