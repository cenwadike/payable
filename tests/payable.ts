import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Payable } from "../target/types/payable";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, Signer, SystemProgram } from "@solana/web3.js";
import { createMint, getAccount, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID, transfer } from "@solana/spl-token";
import { assert } from "chai";

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

  const payer = Keypair.generate();
  const payerSig: Signer = {
    publicKey: payer.publicKey,
    secretKey: payer.secretKey
  }

  const mint = Keypair.generate();

  const [counterPDA, _a] = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("counter"),
    ],
    program.programId
  )

  const [payablePDA, _f] = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("payable"),
      payee.publicKey.toBuffer(),
      payer.publicKey.toBuffer(),
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
      payer.publicKey,
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

  const payeeAta = await getOrCreateAssociatedTokenAccount(connection, payer, token, payee.publicKey);
  const payerAta = await getOrCreateAssociatedTokenAccount(connection, payer, token, payer.publicKey);
  const payableAta = await getOrCreateAssociatedTokenAccount(connection, payer, token, payablePDA, true);

  // transfer some token to payer 
  await transfer(
    connection,
    mint,
    tokenAccount.address,
    payerAta.address,
    mint.publicKey,
    100 
  );

  console.log("-----------------------ADMIN ADDRESS: ", admin.publicKey.toBase58());
  console.log("-----------------------PAYEE ADDRESS: ", payee.publicKey.toBase58());
  console.log("-----------------------PAYER ADDRESS: ", payer.publicKey.toBase58());
  console.log("-----------------------VALID TOKEN MINT: ", token.toBase58());
  console.log("-----------------------COUNTER PDA ADDRESS: ", counterPDA.toBase58());
  console.log("-----------------------PAYER ATA ADDRESS: ", payableAta.address.toBase58());
  console.log("-----------------------PAYABLE ATA ADDRESS: ", payerAta.address.toBase58());
  console.log("-----------------------PAYEE ATA ADDRESS: ", payeeAta.address.toBase58());

  // console.log("-----------------------STARTING INITIALIZATION--------------------------");
  // const initTx = await program.methods.initialize().accounts(
  //   {
  //     counter: counterPDA,
  //     signer: admin.publicKey,
  //     systemProgram: SystemProgram.programId
  //   }
  // ).signers([adminSig]).rpc();
  // console.log("-----------------------INITIALIZATION SUCCESSFUL:", initTx.toString());

  console.log("-----------------------STARTING PAYABLE CREATION--------------------------");
  const createPayableTx = await program.methods.createPayable(
    new anchor.BN(1),
    false,
    new anchor.BN(1),
    new anchor.BN(1),
    new anchor.BN(1)
  ).accounts({
    counter: counterPDA,
    payable: payablePDA,
    signer: payee.publicKey,
    payer: payer.publicKey,
    validTokenMint: token,
    systemProgram: SystemProgram.programId
  }).signers([payeeSig]).rpc()
  console.log("-----------------------PAYABLE CREATION SUCCESSFUL:", createPayableTx.toString());

  console.log("-----------------------STARTING PAYABLE ACCEPTANCE--------------------------");
  const acceptPayableTx = await program.methods.acceptPayable(
    false
  ).accounts({
    payable: payablePDA,
    signer: payer.publicKey,
    payee: payee.publicKey,
    validTokenMint: token,
    payerAta: payerAta.address,
    payableAta: payableAta.address,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId
  }).signers([payerSig]).rpc()
  console.log("-----------------------PAYABLE ACCEPTANCE SUCCESSFUL:", acceptPayableTx.toString());
  
  console.log("-----------------------STARTING PAYABLE CANCELATION--------------------------");
  const cancelPayableTx = await program.methods.cancelPayable(
  ).accounts({
    payable: payablePDA,
    signer: payer.publicKey,
    payee: payee.publicKey,
    validTokenMint: token,
    payeeAta: payeeAta.address,
    payerAta: payerAta.address,
    payableAta: payableAta.address,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId
  }).signers([payerSig]).rpc()
  console.log("-----------------------PAYABLE CANCELATION SUCCESSFUL:", cancelPayableTx.toString());
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
