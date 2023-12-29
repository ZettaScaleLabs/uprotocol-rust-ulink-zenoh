//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use async_std::task;
use std::time;
use uprotocol_sdk::{
    transport::builder::UAttributesBuilder,
    transport::datamodel::UTransport,
    uprotocol::{
        Data, UCode, UEntity, UMessage, UPayload, UPayloadFormat, UPriority, UResource, UStatus,
        UUri,
    },
};
use uprotocol_zenoh_rust::ULinkZenoh;
use zenoh::config::Config;

#[async_std::test]
async fn test_register_and_unregister() {
    let to_test = ULinkZenoh::new(Config::default()).await.unwrap();
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

#[async_std::test]
async fn test_publish_and_subscrib() {
    let target_data = String::from("Hello World!");
    let to_test = ULinkZenoh::new(Config::default()).await.unwrap();
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

    // register the listener
    let uuri_compared = uuri.clone();
    let data_compared = target_data.clone();
    let listener = move |result: Result<UMessage, UStatus>| match result {
        Ok(msg) => {
            if let Data::Value(v) = msg.payload.unwrap().data.unwrap() {
                let value = v.into_iter().map(|c| c as char).collect::<String>();
                assert_eq!(msg.source.unwrap(), uuri_compared);
                assert_eq!(value, data_compared);
            } else {
                panic!("The message should be Data::Value type.");
            }
        }
        Err(ustatus) => println!("Internal Error: {:?}", ustatus),
    };
    let listener_string = to_test
        .register_listener(uuri.clone(), Box::new(listener))
        .await
        .unwrap();

    // create uattributes
    let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs4).build();

    // Publish the data
    let payload = UPayload {
        length: Some(0),
        format: UPayloadFormat::UpayloadFormatText as i32,
        data: Some(Data::Value(target_data.as_bytes().to_vec())),
    };
    to_test
        .send(uuri.clone(), payload, attributes.clone())
        .await
        .unwrap();

    task::sleep(time::Duration::from_millis(1000)).await;

    to_test
        .unregister_listener(uuri.clone(), &listener_string)
        .await
        .unwrap();
}
