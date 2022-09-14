use bevy_assets_bundler::*;

// build.rs
// encryption key: [u8; 16] array
// make sure the key is consistent between build.rs and main.rs
// or follow the example code to share code between build.rs and main.rs
fn main() {
    //let key = [30, 168, 132, 180, 250, 203, 124, 96, 221, 206, 64, 239, 102, 20, 139, 79];
    let mut options = AssetBundlingOptions::default();
    //options.set_encryption_key(key);
    options.encode_file_names = true;
    options.enabled_on_debug_build = true;
    AssetBundler::from(options).build(); //.unwrap();
}
