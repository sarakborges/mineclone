use std::{
    env,
    fs::File,
    io::Write,
    path::PathBuf,
};

use image::{
    codecs::png::PngEncoder,
    imageops::FilterType,
    ImageEncoder,
};

const ICON_SOURCE: &str = "assets/branding/asteria_icon.png";
const ICON_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

fn main() {
    println!("cargo:rerun-if-changed={ICON_SOURCE}");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon_path = build_windows_icon();
    let icon_path = icon_path
        .to_str()
        .expect("generated Asteria icon path should be valid UTF-8");
    let mut resource = winresource::WindowsResource::new();

    resource.set_icon_with_id(icon_path, "1");
    resource
        .compile()
        .expect("Asteria Windows resources should compile");
}

fn build_windows_icon() -> PathBuf {
    let source = image::open(ICON_SOURCE)
        .expect("Asteria icon source should be a valid PNG")
        .into_rgba8();
    let mut frames = Vec::with_capacity(ICON_SIZES.len());

    for size in ICON_SIZES {
        let resized = image::imageops::resize(&source, size, size, FilterType::Lanczos3);
        let mut png = Vec::new();

        PngEncoder::new(&mut png)
            .write_image(
                resized.as_raw(),
                size,
                size,
                image::ExtendedColorType::Rgba8,
            )
            .expect("Asteria icon frame should encode as PNG");
        frames.push((size, png));
    }

    let path = PathBuf::from(
        env::var_os("OUT_DIR").expect("Cargo should provide OUT_DIR to the build script"),
    )
    .join("asteria_icon.ico");
    let mut file = File::create(&path).expect("generated Asteria icon should be writable");
    let directory_size = 6 + frames.len() * 16;
    let mut image_offset = directory_size as u32;

    write_u16(&mut file, 0);
    write_u16(&mut file, 1);
    write_u16(&mut file, frames.len() as u16);

    for (size, png) in &frames {
        let dimension = if *size == 256 { 0 } else { *size as u8 };

        file.write_all(&[dimension, dimension, 0, 0])
            .expect("Asteria icon directory should be writable");
        write_u16(&mut file, 1);
        write_u16(&mut file, 32);
        write_u32(&mut file, png.len() as u32);
        write_u32(&mut file, image_offset);
        image_offset += png.len() as u32;
    }

    for (_, png) in frames {
        file.write_all(&png)
            .expect("Asteria icon frames should be writable");
    }

    path
}

fn write_u16(file: &mut File, value: u16) {
    file.write_all(&value.to_le_bytes())
        .expect("Asteria icon metadata should be writable");
}

fn write_u32(file: &mut File, value: u32) {
    file.write_all(&value.to_le_bytes())
        .expect("Asteria icon metadata should be writable");
}
