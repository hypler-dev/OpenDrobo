fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("../../platform/macos/transport/OpenDroboTransport.c")
            .file("../../platform/macos/transport/CapacityWire.c")
            .file("../../platform/macos/transport/VolumeIcon.m")
            .flag("-fobjc-arc")
            .warnings(true)
            .compile("opendrobo_transport");
        for framework in ["AppKit", "IOKit", "DiskArbitration", "CoreFoundation"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
    println!("cargo:rerun-if-changed=../../platform/macos/transport");
    println!("cargo:rerun-if-changed=../../platform/macos/include");
    tauri_build::build();
}
