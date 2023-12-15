use async_std::task;
use std::time;
use uprotocol_sdk::{
    transport::datamodel::UTransport,
    uprotocol::{UEntity, UMessage, UResource, UUri},
};

use uprotocol_zenoh_rust::ULink;

fn callback(_msg: UMessage) {
    println!("This is callcack");
    // TODO: handling msg
}

#[async_std::main]
async fn main() {
    println!("uProtocol subscriber example");
    let mut subscriber = ULink::new().await.unwrap();

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

    println!("Register the listener...");
    let _listener_str = match subscriber.register_listener(uuri, Box::new(callback)).await {
        Ok(s) => s,
        Err(ustatus) => {
            panic!("{}: {}", ustatus.code, ustatus.message());
        }
    };

    loop {
        //task::sleep(time::Duration::from_millis(1000)).await;
        std::thread::sleep(time::Duration::from_millis(1000));
        println!("Waiting...");
    }

    //if let Err(ustatus) = subscriber.unregister_listener(uuri, &listener_str).await {
    //    panic!("{}: {}", ustatus.code, ustatus.message());
    //}
}
