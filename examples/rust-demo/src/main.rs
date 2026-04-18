mod user;

fn main() {
    let fallback_user = Some("guest");
    let _name = fallback_user.unwrap();

    let macro_example = Some("macro-ok");
    let _suppressed = vec![macro_example.unwrap()];

    println!("lintropy rust-demo");

    let _direct_user = user::sample_direct_user();
    let _built_user = user::User::new("builder-path");
}
