use wow_shared::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let mut args = std::env::args().skip(1);
    let username = args.next().unwrap_or_else(|| "user1".to_string());
    let password = args.next().unwrap_or_else(|| "pass1".to_string());
    let auth_addr = args.next().unwrap_or_else(|| "127.0.0.1:3724".to_string());

    let result = test_client::enter_world(&auth_addr, &username, &password).await?;
    tracing::info!(
        account = %result.account,
        character = %result.character_name,
        realm = %result.realm_address,
        "entered world"
    );
    Ok(())
}
