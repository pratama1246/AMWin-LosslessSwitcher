fn main() {
    println!("cargo:rerun-if-changed=lossless-switcher.rc");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=assets/app.ico");
    embed_resource::compile("lossless-switcher.rc", embed_resource::NONE);
}
