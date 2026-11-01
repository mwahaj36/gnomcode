use super::*;

#[tokio::test]
async fn cloud_requests_need_explicit_key_bound_approval_and_survive_restart() {
    // Run in a child so no test can redirect another test's real SPAWNT_HOME.
    if std::env::var_os("SPAWNT_ACCOUNT_DEVICE_TEST").is_none() {
        let home = tempfile::tempdir().expect("home");
        let result=std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["account_devices::tests::cloud_requests_need_explicit_key_bound_approval_and_survive_restart","--exact","--nocapture"])
            .env("SPAWNT_HOME",home.path()).env("SPAWNT_ACCOUNT_DEVICE_TEST","1").output().expect("child");
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
    {
        let mut state = devices.state.lock().await;
        *state = load_state(owner).await.expect("initialize");
        state.pending.insert(request.id.clone(), request.clone());
    }
    assert_eq!(devices.view().await["devices"][0]["state"], "pending");
    assert!(
        devices.approve(&request.id, &"c".repeat(64)).await.is_err(),
        "approval cannot silently pin a substituted key"
    );
    assert!(devices.state.lock().await.approved.is_empty());
    devices
        .approve(&request.id, &request.public_key)
        .await
        .expect("one action approval");
    let restored = load_state(owner).await.expect("upgrade/restart");
    assert_eq!(
        restored.approved[&request.id].public_key,
        request.public_key
    );
    let other = load_state("20000000-0000-0000-0000-000000000002")
        .await
        .expect("other account");
    assert!(other.approved.is_empty());
    assert_ne!(other.desktop, restored.desktop);
    let cold = AccountDevices::default();
    cold.disconnect_owner(owner)
        .await
        .expect("sign out before the first directory poll");
    assert!(
        load_state(owner)
            .await
            .expect("restart after cold sign-out")
            .approved
            .is_empty()
    );
    devices.revoke(&request.id).await.expect("revoke");
    assert!(
        load_state(owner)
            .await
            .expect("restart after revoke")
            .approved
            .is_empty()
    );
    assert!(root(owner).join(format!("{}.revoked", request.id)).exists());
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
