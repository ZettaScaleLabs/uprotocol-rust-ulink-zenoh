use std::time;
use uprotocol_sdk::{
    transport::datamodel::UTransport,
    uprotocol::{Data, UEntity, UMessage, UResource, UUri},
};

use uprotocol_zenoh_rust::ULink;

fn callback(msg: UMessage) {
    let payload = msg.payload.unwrap();
    let uri = msg.source.unwrap().to_string();
    if let Data::Value(v) = payload.data.unwrap() {
        let value = v.into_iter().map(|c| c as char).collect::<String>();
        println!("Receiving {:?} from {}", value, uri);
    }
}

#[async_std::main]
async fn main() {
    println!("uProtocol subscriber example");
    let subscriber = ULink::new().await.unwrap();

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
        std::thread::sleep(time::Duration::from_millis(1000));
    }

    //if let Err(ustatus) = subscriber.unregister_listener(uuri, &listener_str).await {
    //    panic!("{}: {}", ustatus.code, ustatus.message());
    //}
}
