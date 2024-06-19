use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{Display, Write},
    path::PathBuf,
};

use heck::ToSnakeCase;
use image::RgbaImage;

pub struct ImageAsset {
    pub rgba: RgbaImage,
    pub entry: GatheredEntry,
    pub repeat_x: bool,
    pub repeat_y: bool,
    pub no_pack: bool,
}

pub struct FontAsset {
    pub bytes: Vec<u8>, // ttf file bytes
    pub entry: GatheredEntry,
    pub is_default: bool, // should only be true for one font asset
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetPath {
    segments: Vec<String>,
}

impl Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, e) in self.segments.iter().enumerate() {
            if i != 0 {
                f.write_char('/')?;
            }

            f.write_str(e.as_ref())?;
        }
        Ok(())
    }
}
impl AssetPath {
    pub fn ident(&self) -> &str {
        self.segments.last().unwrap()
    }

    pub fn path(&self) -> &[String] {
        &self.segments[..self.segments.len() - 1]
    }

    fn new() -> Self {
        AssetPath {
            segments: Vec::new(),
        }
    }
}

pub struct GatheredAssets {
    pub images: HashMap<String, ImageAsset>,
    pub fonts: HashMap<String, FontAsset>,
}

pub fn gather_assets(dir: &str) -> GatheredAssets {
    // gather images
    let images_dir = format!("{dir}/images");
    let mut images: HashMap<String, ImageAsset> = HashMap::new();
    println!("gather images:");
    gather_dir_entries(&images_dir, &mut |entry| {
        if entry.extension != "png" {
            return;
        }
        let ident: String = entry.asset_path.ident().to_owned();
        let asset = load_image_asset(entry);
        println!("    image: {ident}");
        match images.entry(ident) {
            Entry::Occupied(other) => {
                panic!(
                    "Duplicate image identifier: {} for {:?} and {:?}",
                    other.key(),
                    asset.entry,
                    other.get().entry
                )
            }
            Entry::Vacant(e) => {
                e.insert(asset);
            }
        }
    });

    // gather fonts
    let fonts_dir = format!("{dir}/fonts");
    let mut fonts: HashMap<String, FontAsset> = HashMap::new();
    println!("gather fonts:");
    gather_dir_entries(&fonts_dir, &mut |entry| {
        if entry.extension != "ttf" {
            return;
        }
        let ident: String = entry.asset_path.ident().to_owned();
        let asset = load_font_asset(entry);
        println!("    font: {ident}");
        match fonts.entry(ident) {
            Entry::Occupied(other) => {
                panic!(
                    "Duplicate font identifier: {} for {:?} and {:?}",
                    other.key(),
                    asset.entry,
                    other.get().entry
                )
            }
            Entry::Vacant(e) => {
                e.insert(asset);
            }
        }
    });

    GatheredAssets { images, fonts }
}

fn load_image_asset(entry: GatheredEntry) -> ImageAsset {
    let bytes: Vec<u8> = std::fs::read(&entry.path).unwrap();
    let rgba = image::load_from_memory(&bytes).unwrap().to_rgba8();

    let mut repeat_x = false;
    let mut repeat_y = false;
    let mut no_pack = false;

    match entry.flags.as_str() {
        "rep" => {
            repeat_x = true;
            repeat_y = true;
        }
        "repx" => {
            repeat_x = true;
        }
        "repy" => {
            repeat_y = true;
        }
        "no" => {
            no_pack = true;
        }
        _ => {}
    };

    ImageAsset {
        rgba,
        entry,
        repeat_x,
        repeat_y,
        no_pack,
    }
}

fn load_font_asset(entry: GatheredEntry) -> FontAsset {
    let bytes: Vec<u8> = std::fs::read(&entry.path).unwrap();
    let is_default = entry.flags == "default";
    FontAsset {
        bytes,
        entry,
        is_default,
    }
}

// fn collect_images(assets_dir: &str) -> HashMap<HashMap<String, ImageAsset>>{

// }

fn gather_dir_entries(dir: &str, f: &mut dyn FnMut(GatheredEntry)) {
    _gather_dir_entries(dir, AssetPath::new(), f);
}

fn _gather_dir_entries(dir: &str, asset_path: AssetPath, f: &mut dyn FnMut(GatheredEntry)) {
    let Ok(dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in dir {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        let file_name = entry.file_name().into_string().unwrap();
        let mut asset_path = asset_path.clone();
        if meta.is_dir() {
            let file_name = file_name.to_snake_case();
            asset_path.segments.push(file_name);
            _gather_dir_entries(entry.path().to_str().unwrap(), asset_path, f);
        } else {
            let mut split = file_name.split('.');
            let file_name = split.next().unwrap().to_snake_case();
            let (flags, ending) = {
                let e1 = split.next().unwrap();
                let e2 = split.next();
                if let Some(e2) = e2 {
                    (e1, e2)
                } else {
                    ("", e1)
                }
            };
            asset_path.segments.push(file_name);
            let entry = GatheredEntry {
                asset_path,
                path: entry.path(),
                flags: flags.to_owned(),
                extension: ending.to_owned(),
            };
            f(entry);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GatheredEntry {
    pub asset_path: AssetPath,
    pub path: PathBuf,
    pub flags: String, // before extension, e.g. "rep" for background.rep.png
    pub extension: String,
}
