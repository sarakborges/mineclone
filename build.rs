fn main() {
    println!("cargo:rerun-if-changed=assets/branding/asteria_icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon_with_id("assets/branding/asteria_icon.ico", "1");
    resource
        .compile()
        .expect("Asteria Windows resources should compile");
}
