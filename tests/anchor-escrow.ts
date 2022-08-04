import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { AnchorEscrow } from '../target/types/anchor_escrow';
import { PublicKey, SystemProgram, Transaction, Connection, Commitment } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, Token, u64 } from "@solana/spl-token";
import { assert } from "chai";

describe('solbet_contract', () => {
  const commitment: Commitment = 'processed';
  const connection = new Connection('https://api.devnet.solana.com', { commitment, wsEndpoint: 'wss://api.devnet.solana.com/' });
  const options = anchor.Provider.defaultOptions();
  const wallet = NodeWallet.local();
  let provider;
  provider = new anchor.Provider(connection, wallet, options);
  provider = anchor.Provider.env();
  console.log("Test Start" /*, provider.connection._rpcEndPoint*/);
  anchor.setProvider(provider);

  const idl = JSON.parse(
    require("fs").readFileSync("./target/idl/solbet_contract.json", "utf8")
  );
  const programId = new anchor.web3.PublicKey("BWVuwvBhFG3nnUexc5CGCzGwELP3eK9oymPCtUiDQXxC");
  const program = new anchor.Program(idl, programId);

  let vault_account_pda = null;
  let vault_account_bump = null;

  let players = [];
  let player_count = 2;
  let payer = new anchor.web3.Keypair();

  for (let i = 0; i < player_count; i++) {
    players[i] = new anchor.web3.Keypair();
  }


  let bet_amount = 100000000 //0.1 sol

  it("Funding players", async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 2000000000), // 2 SOL
      "processed"
    );

    for (let i = 0; i < player_count; i++) {

      let trx = new anchor.web3.Transaction();
      trx.add(anchor.web3.SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: players[i].publicKey,
        lamports: 600000000 //0.6SOL
      }));
      let sig = await anchor.web3.sendAndConfirmTransaction(
        provider.connection,
        trx,
        [payer]
      )
    }
  });
  it("Init Lottery", async () => {
    //event istener
    program.addEventListener('BetResult', (e, s) => {
      console.log('Bet Result In Slot ', s);
      console.log('player', e.player.toString());
      console.log('amount', e.amount.toString());
      console.log('prize_amount', e.prize_amount.toString());
      console.log('ts', e.ts.toString());
    });
    //valut account
    const [_vault_account_pda, _vault_account_bump] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("vault"))],
      program.programId
    );
    vault_account_pda = _vault_account_pda;
    console.log("vault account pda,bump", vault_account_pda.toString(), _vault_account_bump.toString())
    vault_account_bump = _vault_account_bump;


    await program.rpc.init(vault_account_bump, {
      accounts: {
        vaultAccount: vault_account_pda,
        payer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      }
    });
  });

  it("Deposit vault", async () => {
    let balance1 = await provider.connection.getBalance(vault_account_pda);
    // deposit 0.6SOL
    await program.rpc.deposit(
      new anchor.BN(600000000),
      {
        accounts: {
          depositor: payer.publicKey,
          vaultAccount: vault_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [payer]
      }
    );

    let balance2 = await provider.connection.getBalance(vault_account_pda);
    console.log("balance change after deposit", (balance2 - balance1) / anchor.web3.LAMPORTS_PER_SOL);
  });

  it("Withdraw from vault fails without admin", async () => {
    let balance1 = await provider.connection.getBalance(vault_account_pda);
    // withdraw 0.1 SOL
    let error_code = 0;
    try {
      await program.rpc.withdraw(
        new anchor.BN(100000000),
        {
          accounts: {
            withdrawer: payer.publicKey,
            vaultAccount: vault_account_pda,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
          signers: [payer]
        }
      );
    } catch (error) {
      error_code = error.code;
    }
    let balance2 = await provider.connection.getBalance(vault_account_pda);
    console.log("balance change after withdraw", (balance1 - balance2) / anchor.web3.LAMPORTS_PER_SOL);
    assert.ok(error_code === 2003);
  });

  it("Withdraw from vault succeeds with after changing admin.", async () => {
    //set admin
    await program.rpc.setAdmin(
      {
        accounts: {
          admin: provider.wallet.publicKey,
          newAdmin: payer.publicKey,
          vaultAccount: vault_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId
        }
      }
    );

    let balance1 = await provider.connection.getBalance(vault_account_pda);
    // withdraw 0.01 SOL
    let error_code = 0;
    await program.rpc.withdraw(
      new anchor.BN(10000000),
      {
        accounts: {
          withdrawer: payer.publicKey,
          vaultAccount: vault_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [payer]
      }
    );
    let balance2 = await provider.connection.getBalance(vault_account_pda);
    console.log("balance change after withdraw", (balance1 - balance2) / anchor.web3.LAMPORTS_PER_SOL);
  });


  const delay = (time) => new Promise(resolve => setTimeout(resolve, time));

  it("Bet success with enough funds", async () => {
    let balance1 = await provider.connection.getBalance(vault_account_pda);
    const player = players[0];
    //create bet account
    const [bet_account_pda, bet_account_bump] = await PublicKey.findProgramAddress(
      [player.publicKey.toBuffer()],
      program.programId
    );

    await program.rpc.createBetAccount(
      bet_account_bump,
      {
        accounts: {
          player: player.publicKey,
          betAccount: bet_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId
        },
        signers: [player]
      })

    // play
    await program.rpc.bet(
      new anchor.BN(bet_amount),
      {
        accounts: {
          player: player.publicKey,
          vaultAccount: vault_account_pda,
          betAccount: bet_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [player]
      }
    );

    let balance2 = await provider.connection.getBalance(vault_account_pda);
    console.log("balance change after bet", (balance2 - balance1) / anchor.web3.LAMPORTS_PER_SOL);
    // claim prize
    balance1 = await provider.connection.getBalance(bet_account_pda);
    console.log("balance before claim", balance1 / anchor.web3.LAMPORTS_PER_SOL);
    await program.rpc.claimPrize({
      accounts: {
        player: player.publicKey,
        betAccount: bet_account_pda,
        systemProgram: anchor.web3.SystemProgram.programId
      },
      signers: [player]
    });
    balance1 = await provider.connection.getBalance(bet_account_pda);
    console.log("balance after claim", balance1 / anchor.web3.LAMPORTS_PER_SOL);

    // await delay(20000);
  });

});
