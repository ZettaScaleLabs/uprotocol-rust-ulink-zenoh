use uprotocol_zenoh_rust::ULink;

#[async_std::main]
async fn main() {
    println!("uProtocol publisher example");
    let ulink = ULink::new().await.unwrap();
}
