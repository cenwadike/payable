import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Payable } from "../target/types/payable";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, Signer, SystemProgram } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";

const TestProgram = async () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  
  const connection = new Connection(
    'http://127.0.0.1:8899', "confirmed"
  )

  const program = anchor.workspace.Payable as Program<Payable>;
  
  const admin = Keypair.generate();
  const adminSig: Signer = {
    publicKey: admin.publicKey,
    secretKey: admin.secretKey
  }

  const payee = Keypair.generate();
  const payeeSig: Signer = {
    publicKey: payee.publicKey,
    secretKey: payee.secretKey
  }

  const payer1 = Keypair.generate();
  // const payer1Sig: Signer = {
  //   publicKey: payer1.publicKey,
  //   secretKey: payer1.secretKey
  // }

  const payer2 = Keypair.generate();
  const payer3 = Keypair.generate();
  const payer4 = Keypair.generate();
  const payer5 = Keypair.generate();

  const mint = Keypair.generate();

  const [counterPDA, _a] = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("counter"),
    ],
    program.programId
  )

  const [invoicePDA, _f] = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("invoice"),
      payee.publicKey.toBuffer(),
      payer1.publicKey.toBuffer(),
      payer2.publicKey.toBuffer(),
      payer3.publicKey.toBuffer(),
      payer4.publicKey.toBuffer(),
      payer5.publicKey.toBuffer(),
    ],
    program.programId
  )

  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      provider.wallet.publicKey,
      10 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      program.programId,
      10 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      admin.publicKey,
      10 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      payee.publicKey,
      10 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      payer1.publicKey,
      10 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      mint.publicKey,
      10 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );
  
  // create valid token
  const token = await createMint(
    connection,
    mint,
    mint.publicKey,
    null,
    9
  );
  const tokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    mint,
    token,
    mint.publicKey
  )
  await mintTo(
    connection,
    mint,
    token,
    tokenAccount.address,
    mint,
    1000 * 9 // mint 1000
  )

  console.log("-----------------------ADMIN ADDRESS: ", admin.publicKey.toBase58());
  console.log("-----------------------PAYEE ADDRESS: ", payee.publicKey.toBase58());
  console.log("-----------------------VALID TOKEN MINT: ", token.toBase58());
  console.log("-----------------------COUNTER PDA ADDRESS: ", counterPDA.toBase58());

  // console.log("-----------------------STARTING INITIALIZATION--------------------------");
  // const initTx = await program.methods.initialize().accounts(
  //   {
  //     counter: counterPDA,
  //     signer: admin.publicKey,
  //     systemProgram: SystemProgram.programId
  //   }
  // ).signers([adminSig]).rpc();
  // console.log("-----------------------INITIALIZATION SUCCESSFUL:", initTx.toString());

  console.log("-----------------------STARTING INVOICE CREATION--------------------------");

  const createInvoiceTx = await program.methods.createInvoice(
    new anchor.BN(1),
    false,
    new anchor.BN(1),
    new anchor.BN(0),
    new anchor.BN(1)
  ).accounts({
    counter: counterPDA,
    invoice: invoicePDA,
    signer: payee.publicKey,
    payer1: payer1.publicKey,
    payer2: payer2.publicKey,
    payer3: payer3.publicKey,
    payer4: payer4.publicKey,
    payer5: payer5.publicKey,
    validTokenMint: token,
    systemProgram: SystemProgram.programId
  }).signers([payeeSig]).rpc()
  console.log("-----------------------INVOICE CREATION SUCCESSFUL:", createInvoiceTx.toString());
};

const runTest = async () => {
  try {
    await TestProgram();
    process.exit(0);
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
}

runTest()
