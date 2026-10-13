use std::{
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

pub fn build(out: &Path) {
    println!("cargo:rerun-if-changed=scripts/windows_resources.rs");
    println!("cargo:rerun-if-env-changed=RC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        Ok("msvc"),
        "Windows 图标资源目前需要 MSVC 工具链"
    );
    let brand = image::open(out.join("art-brand-planet.png")).unwrap();
    let sizes = [16_u32, 32, 48, 64, 128, 256];
    let images: Vec<_> = sizes
        .iter()
        .map(|&size| {
            let mut data = Cursor::new(Vec::new());
            brand
                .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                .write_to(&mut data, image::ImageFormat::Png)
                .unwrap();
            data.into_inner()
        })
        .collect();
    let mut ico = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&size, image) in sizes.iter().zip(&images) {
        ico.extend_from_slice(&[size as u8, size as u8, 0, 0]);
        ico.extend_from_slice(&1_u16.to_le_bytes());
        ico.extend_from_slice(&32_u16.to_le_bytes());
        ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += image.len() as u32;
    }
    for image in images {
        ico.extend_from_slice(&image);
    }
    std::fs::write(out.join("desktop-meter.ico"), ico).unwrap();
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let numeric = format!("{},0", version.replace('.', ","));
    let resource = format!(
        r#"1 ICON "desktop-meter.ico"
1 VERSIONINFO
 FILEVERSION {numeric}
 PRODUCTVERSION {numeric}
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0
 FILEOS 0x40004
 FILETYPE 1
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "080404b0"
  BEGIN
   VALUE "CompanyName", "曜沧软件\0"
   VALUE "FileDescription", "桌面工作助理\0"
   VALUE "FileVersion", "{version}.0\0"
   VALUE "InternalName", "desktop-meter\0"
   VALUE "OriginalFilename", "desktop-meter.exe\0"
   VALUE "ProductName", "桌面工作助理\0"
   VALUE "ProductVersion", "{version}\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0804, 1200
 END
END
"#
    );
    std::fs::write(out.join("desktop-meter.rc"), resource).unwrap();
    let result = Command::new(resource_compiler())
        .current_dir(out)
        .args([
            "/nologo",
            "/c65001",
            "/fo",
            "desktop-meter.res",
            "desktop-meter.rc",
        ])
        .status()
        .expect("无法执行 Windows SDK 资源编译器 rc.exe");
    assert!(result.success(), "Windows 图标资源编译失败");
    println!(
        "cargo:rustc-link-arg={}",
        out.join("desktop-meter.res").display()
    );
}

fn resource_compiler() -> PathBuf {
    if let Some(rc) = std::env::var_os("RC") {
        return rc.into();
    }
    let kits = PathBuf::from(std::env::var_os("ProgramFiles(x86)").expect("缺少 Windows SDK"))
        .join("Windows Kits/10/bin");
    let mut versions: Vec<_> = std::fs::read_dir(&kits)
        .expect("缺少 Windows SDK 资源编译器")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("10."))
        })
        .collect();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|p| p.join("x64/rc.exe"))
        .find(|p| p.is_file())
        .expect("请安装 Windows SDK，或通过 RC 指定 rc.exe")
}
