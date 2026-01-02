fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

#[test]
fn generated_impls_send_sync() {
    // This will fail to compile if TokenStream isn't Send + Sync
    assert_send::<toml_parser::stream::TokenStream>();
    assert_sync::<toml_parser::stream::TokenStream>();
}
