#[path = "src/build_config.rs"]
mod build_config;

fn main() {
    println!("cargo:rerun-if-env-changed=NEXT_PUBLIC_MARK_TIME_API");
    let icon = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set"))
        .join("mark-time.ico");
    std::fs::write(&icon, icon_bytes()).expect("failed to generate Windows icon");

    let windows = tauri_build::WindowsAttributes::new().window_icon_path(&icon);
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run Tauri build script");

    let icon = icon.display().to_string().replace('\\', "/");
    let api_origin = build_config::api_origin(std::env::var("NEXT_PUBLIC_MARK_TIME_API").ok())
        .unwrap_or_else(|error| panic!("{error}"));
    println!(
        r#"cargo:rustc-env=TAURI_CONFIG={{"app":{{"security":{{"csp":{{"connect-src":"ipc: http://ipc.localhost {api_origin}"}}}}}},"bundle":{{"icon":["{icon}"]}}}}"#
    );
}

fn icon_bytes() -> Vec<u8> {
    const SIZE: u32 = 16;
    const PIXEL_BYTES: u32 = SIZE * SIZE * 4;
    const MASK_BYTES: u32 = SIZE * 4;
    const IMAGE_BYTES: u32 = 40 + PIXEL_BYTES + MASK_BYTES;

    let mut icon = Vec::with_capacity((22 + IMAGE_BYTES) as usize);
    icon.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    icon.extend_from_slice(&[SIZE as u8, SIZE as u8, 0, 0, 1, 0, 32, 0]);
    icon.extend_from_slice(&IMAGE_BYTES.to_le_bytes());
    icon.extend_from_slice(&22_u32.to_le_bytes());
    icon.extend_from_slice(&40_u32.to_le_bytes());
    icon.extend_from_slice(&(SIZE as i32).to_le_bytes());
    icon.extend_from_slice(&((SIZE * 2) as i32).to_le_bytes());
    icon.extend_from_slice(&[1, 0, 32, 0]);
    icon.extend_from_slice(&0_u32.to_le_bytes());
    icon.extend_from_slice(&PIXEL_BYTES.to_le_bytes());
    icon.extend_from_slice(&[0; 16]);

    for y in (0..SIZE).rev() {
        for x in 0..SIZE {
            let on_ring = !(3..=12).contains(&x) || !(3..=12).contains(&y);
            let on_hand = (x == 8 && (4..=8).contains(&y)) || (y == 8 && (8..=12).contains(&x));
            icon.extend_from_slice(if on_ring || on_hand {
                &[8, 179, 234, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
    }

    icon.resize((22 + IMAGE_BYTES) as usize, 0);
    icon
}
