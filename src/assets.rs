pub struct Raster {
    pub name: &'static str,
    pub size: [usize; 2],
    pub rgba: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/artwork_data.rs"));

pub fn get(name: &str) -> &'static Raster {
    CATALOG
        .iter()
        .find(|asset| asset.name == name)
        .expect("已接入的素材必须存在")
}

pub fn window_icon() -> std::sync::Arc<eframe::egui::IconData> {
    static ICON: std::sync::OnceLock<std::sync::Arc<eframe::egui::IconData>> =
        std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        let image = get("brand-planet");
        std::sync::Arc::new(eframe::egui::IconData {
            rgba: image.rgba.to_vec(),
            width: image.size[0] as u32,
            height: image.size[1] as u32,
        })
    })
    .clone()
}
