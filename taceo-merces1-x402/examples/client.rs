use alloy::signers::local::PrivateKeySigner;
use reqwest::Client;
use taceo_merces1_x402::V2Eip155ConfidentialClient;
use x402_reqwest::{ReqwestWithPayments, ReqwestWithPaymentsBuild, X402Client};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let private_key = std::env::var("PRIVATE_KEY").unwrap_or(
        "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6".to_string(),
    );
    let signer: PrivateKeySigner = private_key.parse()?;
    let server_url = std::env::var("SERVER_URL").unwrap_or("http://localhost:8081".to_string());

    let x402_client = X402Client::new().register(V2Eip155ConfidentialClient::new(signer));

    let http_client = Client::new().with_payments(x402_client).build();

    let response = http_client
        .get(format!("{server_url}/api/protected"))
        .send()
        .await?;

    println!("Status: {}", response.status());
    println!("Body: {}", response.text().await?);

    Ok(())
}
