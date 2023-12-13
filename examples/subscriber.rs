use uprotocol_zenoh_rust::ULink;

#[async_std::main]
async fn main() {
    println!("uProtocol subscriber example");
    let ulink = ULink::new().await.unwrap();
}
