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
