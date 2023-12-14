use std::{thread, time};
use uprotocol_sdk::{
    transport::datamodel::UTransport,
    uprotocol::{UEntity, UMessage, UResource, UUri},
};

use uprotocol_zenoh_rust::ULink;

fn callback(msg: UMessage) {
    println!("This is callcack");
    // TODO: handling msg
}

#[async_std::main]
async fn main() {
    println!("uProtocol subscriber example");
    let subscriber = ULink::new().await.unwrap();

    // create uuri
    let uuri = UUri {
        entity: Some(UEntity {
            name: "body.access".to_string(),
            ..Default::default()
        }),
        resource: Some(UResource {
            name: "door".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    println!("Register the listener...");
    let listener_str = match subscriber.register_listener(uuri, Box::new(callback)).await {
        Ok(s) => s,
        Err(ustatus) => {
            panic!("{}: {}", ustatus.code, ustatus.message());
        }
    };

    loop {
        thread::sleep(time::Duration::from_millis(1000));
    }

    if let Err(ustatus) = subscriber.unregister_listener(uuri, &listener_str).await {
        panic!("{}: {}", ustatus.code, ustatus.message());
    }
}
