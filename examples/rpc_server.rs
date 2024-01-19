//
// Copyright (c) 2024 ZettaScale Technology
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
use async_std::task::{self, block_on};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use std::time;
use uprotocol_sdk::{
    rpc::RpcServer,
    transport::datamodel::UTransport,
    uprotocol::{Data, UEntity, UMessage, UMessageType, UPayload, UPayloadFormat, UStatus, UUri},
    uri::builder::resourcebuilder::UResourceBuilder,
};
use uprotocol_zenoh_rust::ULinkZenoh;
use zenoh::config::Config;

#[async_std::main]
async fn main() {
    println!("uProtocol RPC server example");
    let rpc_server = Arc::new(Mutex::new(
        ULinkZenoh::new(Config::default()).await.unwrap(),
    ));

    // create uuri
    // TODO: Need to check whether the way to create ID is correct?
    let uuri = UUri {
        entity: Some(UEntity {
            name: "test_rpc.app".to_string(),
            version_major: Some(1),
            id: Some(1234),
            ..Default::default()
        }),
        resource: Some(UResourceBuilder::for_rpc_request(
            Some("getTime".to_string()),
            Some(5678),
        )),
        ..Default::default()
    };

    let rpc_server_cloned = rpc_server.clone();
    let callback = move |result: Result<UMessage, UStatus>| {
        match result {
            Ok(msg) => {
                let UMessage {
                    source,
                    attributes,
                    payload,
                } = msg;
                // Get the UUri
                let uuri = source.unwrap();
                // Build the payload to send back
                if let Data::Value(v) = payload.unwrap().data.unwrap() {
                    let value = v.into_iter().map(|c| c as char).collect::<String>();
                    println!("Receive {} from {}", value, uuri.to_string());
                }
                // Get current time
                let upayload = UPayload {
                    length: Some(0),
                    format: UPayloadFormat::UpayloadFormatText as i32,
                    data: Some(Data::Value(format!("{}", Utc::now()).as_bytes().to_vec())),
                };
                // Set the attributes type to Response
                let mut uattributes = attributes.unwrap();
                uattributes.set_type(UMessageType::UmessageTypeResponse);
                // Send back result
                block_on(
                    rpc_server_cloned
                        .lock()
                        .unwrap()
                        .send(uuri, upayload, uattributes),
                )
                .unwrap();
            }
            Err(ustatus) => {
                println!("Internal Error: {:?}", ustatus);
            }
        }
    };

    println!("Register the listener...");
    rpc_server
        .lock()
        .unwrap()
        .register_rpc_listener(uuri, Box::new(callback))
        .await
        .unwrap();

    loop {
        task::sleep(time::Duration::from_millis(1000)).await;
    }
}
