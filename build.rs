#[path = "scripts/visual_assets.rs"]
mod visual_assets;

fn main() {
    println!("cargo:rerun-if-changed=scripts/visual_assets.rs");
    let folder = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    visual_assets::build(&folder);
    println!("cargo:rerun-if-changed=assets/pets/atlas.png");
    let atlas = image::open("assets/pets/atlas.png")
        .expect("宠物图集必须存在")
        .to_rgba8();
    assert!(
        atlas.width().is_multiple_of(3) && atlas.height().is_multiple_of(2),
        "宠物图集必须为三列两行"
    );
    assert!(
        atlas.pixels().any(|pixel| pixel[3] == 0),
        "宠物图集必须包含透明背景"
    );
    let cell = [atlas.width() / 3, atlas.height() / 2];
    for row in 0..2 {
        for column in 0..3 {
            let sprite = image::imageops::crop_imm(
                &atlas,
                column * cell[0],
                row * cell[1],
                cell[0],
                cell[1],
            )
            .to_image();
            let sprite =
                image::imageops::resize(&sprite, 96, 96, image::imageops::FilterType::Lanczos3);
            std::fs::write(
                folder.join(format!("pet-{column}-{row}.rgba")),
                sprite.as_raw(),
            )
            .unwrap();
        }
    }
}
