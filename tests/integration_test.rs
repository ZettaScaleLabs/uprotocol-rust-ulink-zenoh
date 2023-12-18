use uprotocol_sdk::{
    transport::datamodel::UTransport,
    uprotocol::{UCode, UEntity, UResource, UStatus, UUri},
};
use uprotocol_zenoh_rust::ULink;

#[async_std::test]
async fn test_register_test() {
    let to_test = ULink::new().await.unwrap();
    // create uuri
    let uuri = UUri {
        entity: Some(UEntity {
            name: "body.access".to_string(),
            version_major: Some(1),
            ..Default::default()
        }),
        resource: Some(UResource {
            name: "door".to_string(),
            instance: Some("front_left".to_string()),
            message: Some("Door".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let listener_string = to_test
        .register_listener(uuri.clone(), Box::new(|_| {}))
        .await
        .unwrap();
    // Should succeed
    let result = to_test
        .unregister_listener(uuri.clone(), &listener_string)
        .await
        .unwrap();
    assert_eq!(result, ());
    // Should fail
    let result = to_test
        .unregister_listener(uuri.clone(), &listener_string)
        .await;
    assert_eq!(
        result,
        Err(UStatus::fail_with_code(
            UCode::InvalidArgument,
            "Listener not exists"
        ))
    )
}
