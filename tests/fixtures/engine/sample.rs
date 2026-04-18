fn main() {
    let client = make_client();
    let user = client.unwrap();
    println!("debug: {:?}", user);
    // TODO: remove before merge
}
