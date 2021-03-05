use std::{
    env,
    error::Error,
    fs,
    io::{self},
    time::Duration,
};

use algo_rust_sdk::{
    account::Account,
    algod::models::TransactionID,
    transaction::{BaseTransaction, Payment, Transaction, TransactionType},
    AlgodClient, MicroAlgos, Round,
};

use algorand_compounder::*;

pub fn get_algod_address_token_pair() -> Result<(String, String), Box<dyn Error>> {
    let algorand_data = env::var("ALGORAND_DATA")?;
    let algod_address = format!(
        "http://{}",
        fs::read_to_string(format!("{}/algod.net", algorand_data))?.trim()
    );
    let algod_token = fs::read_to_string(format!("{}/algod.token", algorand_data))?;
    Ok((algod_address, algod_token))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut auto_payment_count = 0;

    let (algod_address, algod_token) = get_algod_address_token_pair()?;
    let mut account_mnemonic = String::new();
    let stdin = io::stdin();

    println!("input the 25 word mnemonic for your account:");
    stdin.read_line(&mut account_mnemonic)?;

    let algod_client = AlgodClient::new(&algod_address, &algod_token);
    let bank_acc = Account::from_mnemonic(account_mnemonic.trim())?;
    let bank_addr = bank_acc.address();

    loop {
        let transaction_params = algod_client.transaction_params()?;
        let genesis_id = transaction_params.genesis_id;
        let genesis_hash = transaction_params.genesis_hash;
        let acc_info = algod_client.account_information(&bank_addr.encode_string())?;

        let balance = acc_info.amount.0 as f64 / 1E6;
        println!("current algo = {}", balance);

        let model = AlgoInterestModel::new(CompoundModelCoefs::new(1., 0.069, 0.001, balance));

        let delay = match model.get_ideal_reward_wait_time() {
            Some(seconds) => seconds,
            None => 3600. * 24., // if for some reason there is an error, it just waits a dat
        };

        let base_transaction = BaseTransaction {
            sender: bank_addr.clone(),
            first_valid: transaction_params.last_round,
            last_valid: transaction_params.last_round + 1000,
            note: format!(
                "This was an automated payment for compounding count:{}",
                auto_payment_count,
            )
            .as_bytes()
            .iter()
            .map(|&a| a)
            .collect(),
            genesis_id,
            genesis_hash,
        };

        let payment = Payment {
            amount: MicroAlgos(0),
            receiver: bank_addr.clone(),
            close_remainder_to: None,
        };

        let transaction = Transaction::new_flat_fee(
            base_transaction,
            MicroAlgos(1000),
            TransactionType::Payment(payment),
        );

        let signed_transaction = bank_acc.sign_transaction(&transaction)?;
        println!("signed transaction");

        // Broadcast the transaction to the network
        // Note this transaction will get rejected because the accounts do not have any tokens
        let send_response = algod_client.send_transaction(&signed_transaction)?;
        println!("Transaction ID: {}", send_response.tx_id);

        match confirm_transaction(&algod_client, &send_response, 10) {
            Ok(_) => {
                println!(
                    "Transaction success, sleeping by {} seconds or {} days",
                    delay,
                    delay / (24.0 * 3600.0)
                );
                std::thread::sleep(Duration::from_secs_f64(delay));
                auto_payment_count += 1;
            }
            Err(kind) => println!("Transactin failed:{}", kind),
        }
    }

    Ok(())
}

pub fn confirm_transaction(
    algod_client: &AlgodClient,
    send_response: &TransactionID,
    timeout: u64,
) -> Result<(), Box<dyn Error>> {
    let status = algod_client.status()?;
    let start_round: Round = status.last_round + 1;
    let mut current_round: Round = start_round;

    while current_round.0 < (start_round + timeout).0 {
        let pending_info = algod_client.pending_transaction_information(&send_response.tx_id)?;
        if pending_info.round.is_some() && pending_info.round.unwrap() > 0 {
            return Ok(());
        } else {
            if pending_info.pool_error.len() > 0 {
                return Err(ConfirmationError::new(format!(
                    "Transaction Rejected:{}",
                    pending_info.pool_error
                )));
            }
        }
        algod_client.status_after_block(current_round)?;
        current_round = current_round + 1;
    }

    Err(ConfirmationError::new(String::from("Timeout exceeded")))
}
