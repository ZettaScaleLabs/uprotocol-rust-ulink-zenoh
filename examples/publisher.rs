use async_std::task;
use std::time;
use uprotocol_sdk::{
    transport::builder::UAttributesBuilder,
    transport::datamodel::UTransport,
    uprotocol::{Data, UEntity, UPayload, UPayloadFormat, UPriority, UResource, UUri},
};
use uprotocol_zenoh_rust::ULink;

#[async_std::main]
async fn main() {
    println!("uProtocol publisher example");
    let publisher = ULink::new().await.unwrap();

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

    // create uattributes
    let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs4).build();

    let mut cnt: u64 = 0;
    loop {
        let data = format!("{}", cnt);
        let value = data.as_bytes().to_vec();
        let payload = UPayload {
            length: Some(0),
            format: UPayloadFormat::UpayloadFormatText as i32,
            data: Some(Data::Value(value)),
        };
        println!("Sending {} to {}...", data, uuri.to_string());
        if let Err(ustatus) = publisher
            .send(uuri.clone(), payload, attributes.clone())
            .await
        {
            panic!("{}: {}", ustatus.code, ustatus.message());
        }
        task::sleep(time::Duration::from_millis(1000)).await;
        cnt += 1;
    }
}
