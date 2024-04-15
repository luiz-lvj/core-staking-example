import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { CoreStakingExample } from "../target/types/core_staking_example";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults"
import { createSignerFromKeypair, generateSigner, signerIdentity, publicKey } from "@metaplex-foundation/umi";
import { MPL_CORE_PROGRAM_ID, mplCore, createV1, createCollectionV1, transferV1 } from '@metaplex-foundation/mpl-core'

describe("core-staking-example", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const connection = provider.connection;
  const wallet = anchor.Wallet.local();
  const program = anchor.workspace.CoreStakingExample as Program<CoreStakingExample>;

  // Metaplex Setup
  const umi = createUmi(connection.rpcEndpoint);
  let umiKeypair = umi.eddsa.createKeypairFromSecretKey(wallet.payer.secretKey);
  const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
  umi.use(signerIdentity(signerKeypair)).use(mplCore());

  // Helpers
  function wait(ms: number) {
    return new Promise( resolve => setTimeout(resolve, ms) );
  }

  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block
    })
    return signature
  }

  const log = async(signature: string): Promise<string> => {
    console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`);
    return signature;
  }

  // Variables
  const collection = generateSigner(umi);
  const asset = generateSigner(umi);
  const newOwner = publicKey(generateSigner(umi));
  const stakingAccount = PublicKey.findProgramAddressSync([Buffer.from("staking_account"), wallet.publicKey.toBuffer()], program.programId)[0];

  it("Creates Assets and Collections", async () => {

    await createCollectionV1(umi, {
      collection,
      name: 'My Collection',
      uri: 'https://example.com/my-collection.json',
    }).sendAndConfirm(umi)

    await createV1(umi, {
      asset: asset,
      name: 'My Nft',
      uri: 'https://example.com/my-nft.json',
      collection: collection.publicKey,
    }).sendAndConfirm(umi)

  });

  it("Create Staking Account", async () => {
    await program.methods.createStakingAccount()
    .accounts({
      stakingAccount
    })
    .signers([wallet.payer])
    .rpc({skipPreflight: true}).then(log).then(confirm);
  });

  it("Stake", async () => {
      await program.methods.stake()
      .accounts({
        asset: asset.publicKey,
        collection: collection.publicKey,
        stakingAccount,
        coreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([wallet.payer])
      .rpc({skipPreflight: true}).then(log).then(confirm);
  });

  it("Unstake", async () => {
    //await wait(10_000);
    
    await program.methods.unstake()
    .accounts({
      asset: asset.publicKey,
      collection: collection.publicKey,
      stakingAccount,
      coreProgram: MPL_CORE_PROGRAM_ID,
    })
    .signers([wallet.payer])
    .rpc({skipPreflight: true}).then(log).then(confirm);
  });
});
