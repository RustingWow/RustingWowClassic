use std::time::Duration;

use tokio::net::TcpListener;
use tokio::time::sleep;

#[tokio::test]
async fn user1_reaches_login_verify_world() {
    let login = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let world = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let login_addr = login.local_addr().unwrap();
    let http_addr = http.local_addr().unwrap();
    let world_addr = world.local_addr().unwrap();

    tokio::spawn(async move {
        auth_server::serve_with_listeners(login, http, world_addr.to_string())
            .await
            .unwrap();
    });
    tokio::spawn(async move {
        world_server::serve_with_listener(world, format!("http://{http_addr}"), false)
            .await
            .unwrap();
    });

    wait_for_health(&format!("http://{http_addr}")).await;

    let result = test_client::enter_world(&login_addr.to_string(), "user1", "pass1")
        .await
        .expect("enter world");

    assert_eq!(result.account, "USER1");
    assert_eq!(result.character_name, "Userone");
    assert_eq!(result.realm_address, world_addr.to_string());
}

async fn wait_for_health(url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client
            .get(format!("{url}/health"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }
    panic!("auth internal HTTP did not become ready at {url}");
}
