use std::{thread, time};
use uprotocol_sdk::{
    transport::builder::UAttributesBuilder,
    transport::datamodel::UTransport,
    uprotocol::{UEntity, UPayload, UPriority, UResource, UUri},
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
            ..Default::default()
        }),
        resource: Some(UResource {
            name: "door".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    // create uattributes
    let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs4).build();

    let mut cnt: u64 = 0;
    loop {
        // create the payload
        let payload = UPayload::default();
        // TODO: Create UPayload
        //let data = Any
        //let payload = UPayload {
        //    length: (),
        //    format: UPayloadFormat::UpayloadFormatText as i32,
        //    data: Some(Data::Value()),
        //};
        thread::sleep(time::Duration::from_millis(1000));
        println!("Sending data {} to {}...", cnt, uuri.to_string());
        if let Err(ustatus) = publisher
            .send(uuri.clone(), payload, attributes.clone())
            .await
        {
            panic!("{}: {}", ustatus.code, ustatus.message());
        }
        cnt += 1;
    }
}
