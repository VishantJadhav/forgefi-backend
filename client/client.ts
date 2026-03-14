import { BN } from "@coral-xyz/anchor";

console.log("Starting ForgeFi Week 1 Escrow Test...");

// 1. Derive the PDA Vault address using the user's wallet
const [userStakePDA] = web3.PublicKey.findProgramAddressSync(
  [Buffer.from("stake"), pg.wallet.publicKey.toBuffer()],
  pg.program.programId
);
console.log("Vault Address:", userStakePDA.toBase58());

// 2. Initialize the Routine (Deposit 1 Devnet SOL as stake)
console.log("\n--- STEP 1: Locking Routine & Depositing Stake ---");
const stakeAmount = new BN(1 * web3.LAMPORTS_PER_SOL);
const daysCommitted = 6; // 6-day split

const tx1 = await pg.program.methods
  .initializeRoutine(stakeAmount, daysCommitted)
  .accounts({
    userStake: userStakePDA,
  })
  .rpc();
console.log("Success! Transaction Hash:", tx1);

// 3. Verify Workout (Update Timestamp)
console.log("\n--- STEP 2: Verifying Daily Workout ---");
const tx2 = await pg.program.methods
  .verifyWorkout()
  .accounts({
    userStake: userStakePDA,
  })
  .rpc();
console.log("Success! Transaction Hash:", tx2);

// 4. Resolve Stake (Unlock Vault and Refund)
console.log("\n--- STEP 3: Sprint Complete. Refunding Stake ---");
const tx3 = await pg.program.methods
  .resolveStake()
  .accounts({
    userStake: userStakePDA,
  })
  .rpc();
console.log("Success! Vault closed and funds returned. Transaction Hash:", tx3);