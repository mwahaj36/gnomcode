use super::*;

#[tokio::test]
async fn native_consent_grants_all_account_devices_across_desktops_and_survives_restart() {
    // Run in a child so no test can redirect another test's real GNOMCODE_HOME.
    if std::env::var_os("GNOMCODE_ACCOUNT_DEVICE_TEST").is_none() {
        let home = tempfile::tempdir().expect("home");
        let result=std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["account_devices::tests::native_consent_grants_all_account_devices_across_desktops_and_survives_restart","--exact","--nocapture"])
            .env("GNOMCODE_HOME",home.path()).env("GNOMCODE_ACCOUNT_DEVICE_TEST","1").output().expect("child");
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let owner = "10000000-0000-0000-0000-000000000001";
    let devices = AccountDevices::default();
    let request = Request {
        id: "a".repeat(32),
        public_key: "b".repeat(64),
        name: "Phone browser".into(),
        revoked: false,
        offer: None,
        state: String::new(),
    };
    let mut first = load_state(owner).await.expect("first desktop");
    first.remote_enabled = true;
    assert!(
        !first.admit(&request),
        "existing links must not silently gain account-wide access"
    );
    devices
        .enable_account_access(owner)
        .await
        .expect("confirm native account access once");
    first = load_state(owner).await.expect("consent survives restart");
    first.remote_enabled = true;
    assert!(
        first.admit(&request),
        "signed-in phone connects without per-desktop approval"
    );
    save_state(&first).await.expect("save connection");
    let mut second = State {
        owner: owner.into(),
        desktop: "2".repeat(32),
        account_access: true,
        remote_enabled: true,
        ..State::default()
    };
    assert!(
        second.admit(&request),
        "the same signed-in phone also connects to another consented native app"
    );
    let second_phone = Request {
        id: "c".repeat(32),
        public_key: "d".repeat(64),
        ..request.clone()
    };
    assert!(
        first.admit(&second_phone),
        "another signed-in device needs no separate approval either"
    );
    first.remote_enabled = false;
    assert!(
        !first.admit(&second_phone),
        "account-side revocation blocks all clients"
    );
    let other = load_state("20000000-0000-0000-0000-000000000002")
        .await
        .expect("other account");
    assert!(!other.account_access);
    assert!(other.approved.is_empty());
    // Execute the real publication path after a restart against a revoked directory row.
    let writes = Arc::new(Mutex::new(Vec::<(String, String, Value)>::new()));
    let capture = writes.clone();
    let app = axum::Router::new().fallback(axum::routing::any(
        move |request: axum::extract::Request| {
            let capture = capture.clone();
            async move {
                let method = request.method().to_string();
                let prefer = request
                    .headers()
                    .get("prefer")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_owned();
                let bytes = axum::body::to_bytes(request.into_body(), 4096)
                    .await
                    .expect("body");
                let value = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
                capture.lock().await.push((method.clone(), prefer, value));
                axum::Json(if method == "GET" {
                    json!([{"remote_enabled":false}])
                } else {
                    json!({})
                })
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("cloud fixture");
    let address = listener.local_addr().expect("address");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    let restored = Arc::new(AccountDevices::default());
    let auth = CloudAuth {
        user_id: owner.into(),
        access_token: "synthetic".into(),
        public_key: "synthetic".into(),
        base_url: format!("http://{address}"),
    };
    let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
    for _ in 0..2 {
        restored
            .poll(&reqwest::Client::new(), &auth, db.clone())
            .await
            .expect("poll revoked host");
    }
    assert_eq!(restored.view().await["remote_enabled"], false);
    for (method, prefer, payload) in writes.lock().await.iter() {
        if method == "POST" {
            assert!(
                prefer.contains("ignore-duplicates"),
                "registration must not overwrite a revoked native app"
            );
        }
        if method == "PATCH" {
            assert!(
                payload.get("remote_enabled").is_none(),
                "heartbeat must not re-enable account access after restart"
            );
        }
    }
    restored
        .set_account_access(&reqwest::Client::new(), &auth, true)
        .await
        .expect("explicit native confirmation may enable again");
    assert_eq!(restored.view().await["remote_enabled"], true);
    assert!(
        writes
            .lock()
            .await
            .iter()
            .any(|(method, _, payload)| method == "PATCH" && payload["remote_enabled"] == true),
        "only the explicit consent operation writes an enable"
    );
    server.abort();
    let cold = AccountDevices::default();
    cold.disconnect_owner(owner)
        .await
        .expect("sign out before the first directory poll");
    assert!(
        !load_state(owner)
            .await
            .expect("restart after sign-out")
            .account_access
    );
}

#[test]
fn device_request_rejects_paths_and_unbounded_labels() {
    let mut request = Request {
        id: "a".repeat(32),
        public_key: "b".repeat(64),
        name: "Phone browser".into(),
        revoked: false,
        offer: None,
        state: String::new(),
    };
    assert!(request.valid());
    request.id = "../outside".into();
    assert!(!request.valid());
    request.id = "a".repeat(32);
    request.name = "x".repeat(81);
    assert!(!request.valid());
    assert!(!uuid_owner("../../not-an-account"));
}

#[tokio::test]
async fn remove_device_deletes_approved_connection_and_persists() {
    let owner = "30000000-0000-0000-0000-000000000003";
    let devices = AccountDevices::default();
    devices.enable_account_access(owner).await.expect("enable");
    let device_id = "e".repeat(32);
    let request = Request {
        id: device_id.clone(),
        public_key: "f".repeat(64),
        name: "Phone browser 1234".into(),
        revoked: false,
        offer: None,
        state: String::new(),
    };
    let mut state = load_state(owner).await.expect("load");
    state.remote_enabled = true;
    assert!(state.admit(&request));
    save_state(&state).await.expect("save");

    let auth = CloudAuth {
        user_id: owner.into(),
        access_token: "synthetic".into(),
        public_key: "synthetic".into(),
        base_url: "http://127.0.0.1:9".into(),
    };
    let client = reqwest::Client::new();
    devices
        .remove_device(&client, &auth, &device_id)
        .await
        .expect("remove device");

    let updated = load_state(owner).await.expect("reload");
    assert!(!updated.approved.contains_key(&device_id));
}
