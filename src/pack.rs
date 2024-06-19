use std::collections::{HashMap, HashSet};

use crate::{
    font::font_to_sdf_font,
    gather::{FontAsset, GatheredAssets, ImageAsset},
};
use glam::{uvec2, UVec2};
use image::{GenericImage, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureTile {
    pub atlas: String,
    pub min: UVec2,
    pub max: UVec2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TextureFlags {
    pub repeat_x: bool,
    pub repeat_y: bool,
}

impl TextureFlags {
    pub const REPEAT: TextureFlags = TextureFlags {
        repeat_x: true,
        repeat_y: true,
    };
    pub const REPEAT_X: TextureFlags = TextureFlags {
        repeat_x: true,
        repeat_y: false,
    };
    pub const NO_REPEAT: TextureFlags = TextureFlags {
        repeat_x: false,
        repeat_y: false,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackedAssets {
    pub textures: Vec<(String, TextureFlags)>, // file names
    pub tiles: HashMap<String, TextureTile>,
    pub fonts: Vec<String>, // font names
    pub default_font: String,
}

pub fn pack_assets(gathered: &GatheredAssets, out_path: &str) {
    _ = std::fs::remove_dir_all(out_path);
    std::fs::create_dir(out_path).unwrap();

    let mut packed = PackedAssets::default();

    let (textures, tiles) = make_texture_atlases(&gathered.images);
    packed.tiles = tiles;
    for (i, (rgba, flags)) in textures.iter().enumerate() {
        let texture_name = atlas_name(i);
        rgba.save(format!("{out_path}/{texture_name}.png")).unwrap();
        packed.textures.push((texture_name, *flags));
    }
    let mut default_font: Option<String> = None;
    for (name, font) in gathered.fonts.iter() {
        if font.is_default {
            default_font = Some(name.clone());
        }

        let (sdf_font, sdf_image) = font_to_sdf_font(font);

        std::fs::write(
            format!("{out_path}/{}.sdf_font.json", name),
            serde_json::to_string(&sdf_font).unwrap(),
        )
        .unwrap();
        sdf_image
            .save(format!("{out_path}/{}.sdf_font.png", name))
            .unwrap();

        packed.fonts.push(name.clone());
    }
    packed.default_font = default_font.expect("there should be one default font");

    for (name, asset) in gathered.images.iter() {
        if asset.no_pack {
            let (w, h) = asset.rgba.dimensions();
            packed.textures.push((
                name.clone(),
                TextureFlags {
                    repeat_x: asset.repeat_x,
                    repeat_y: asset.repeat_y,
                },
            ));
            packed.tiles.insert(
                name.clone(),
                TextureTile {
                    atlas: name.clone(),
                    min: uvec2(0, 0),
                    max: uvec2(w, h),
                },
            );
            asset.rgba.save(format!("{out_path}/{name}.png")).unwrap();
        }
    }

    std::fs::write(
        format!("{out_path}/packed.json"),
        serde_json::to_string(&packed).unwrap(),
    )
    .unwrap();
}

//  returns pad_x and pad_y
fn pad_for_image_asset(asset: &ImageAsset) -> (u32, u32) {
    let pad_x: u32;
    let pad_y: u32;
    if asset
        .entry
        .asset_path
        .path()
        .last()
        .is_some_and(|e| e == "characters")
    {
        let (w, h) = asset.rgba.dimensions();
        pad_x = w / 8;
        pad_y = h / 8;
    } else {
        pad_x = 2;
        pad_y = 2;
    }

    (pad_x, pad_y)
}

pub fn make_texture_atlases(
    images: &HashMap<String, ImageAsset>,
) -> (Vec<(RgbaImage, TextureFlags)>, HashMap<String, TextureTile>) {
    let atlas_w: u32 = 1024; // todo! incorporate things like max_width and min_width here...
    let atlas_h: u32 = 1024;

    let mut atlases: Vec<(RgbaImage, TextureFlags)> = vec![];

    let mut tiles: HashMap<String, TextureTile> = HashMap::new();

    // let allocator = tgf::ext::etagere::AtlasAllocator::new(size);
    let mut sorted: Vec<(&ImageAsset, bool)> = images
        .values()
        .filter(|e| !e.no_pack)
        .map(|e| (e, false))
        .collect();
    sorted.sort_by(|a, b| {
        match a
            .0
            .entry
            .asset_path
            .path()
            .cmp(&b.0.entry.asset_path.path())
        {
            std::cmp::Ordering::Equal => {
                let h1 = a.0.rgba.height();
                let h2 = b.0.rgba.height();
                h2.cmp(&h1)
            }
            e => e,
        }
    });

    // first handle the images that need some sort of tiling:

    let mut min_w: u32 = u32::MAX; // all of these min max only across non-repeat images
    let mut min_h: u32 = u32::MAX;
    let mut max_w: u32 = 0;
    let mut max_h: u32 = 0;
    let mut rep_x_buckets: HashMap<u32, Vec<(usize, u32)>> = HashMap::new(); // maps width to indices and thierheight
    let mut rep_y_buckets: HashMap<u32, Vec<(usize, u32)>> = HashMap::new(); // maps height to indices and their and width

    for (i, (e, allocated)) in sorted.iter_mut().enumerate() {
        let (w, h) = e.rgba.dimensions();

        if e.repeat_x && e.repeat_y {
            // if repx and repy give it its own texture
            tiles.insert(
                e.entry.asset_path.ident().to_owned(),
                TextureTile {
                    atlas: e.entry.asset_path.ident().to_string(),
                    min: uvec2(0, 0),
                    max: uvec2(w, h),
                },
            );
            atlases.push((e.rgba.clone(), TextureFlags::REPEAT));
            *allocated = true;
        } else if e.repeat_x {
            rep_x_buckets.entry(w).or_default().push((i, h));
        } else if e.repeat_y {
            rep_y_buckets.entry(h).or_default().push((i, w));
        } else {
            if h < min_h {
                min_h = h;
            }
            if w < min_w {
                min_w = w;
            }
            if h > max_h {
                max_h = h;
            }
            if w > max_w {
                max_w = w;
            }
        }
    }

    for (width, entries) in rep_x_buckets.iter() {
        let pad = 2;
        let entries_height: u32 = entries.iter().map(|e| e.1 + pad).sum::<u32>();

        let mut asset_paths_of_bucket: HashSet<Vec<String>> = HashSet::new();

        let height = next_pow2_number(entries_height as usize).max(256) as u32;
        let mut atlas: RgbaImage = RgbaImage::new(*width, height);

        let mut y: u32 = 0;

        // allocate the vertical strips:
        for (i, h) in entries.iter() {
            let (asset, allocated) = &mut sorted[*i];
            *allocated = true;
            atlas.copy_from(&asset.rgba, 0, y).unwrap();

            let tile = TextureTile {
                atlas: atlas_name(atlases.len()),
                min: uvec2(0, y),
                max: uvec2(atlas.width(), y + *h),
            };

            y += *h + pad;
            tiles.insert(asset.entry.asset_path.ident().to_owned(), tile);

            asset_paths_of_bucket.insert(asset.entry.asset_path.path().to_vec());
        }

        // try to put some images around in the remaining height:
        let remaining_height = height - entries_height;
        if remaining_height >= min_h {
            let mut remaining_size_allocator =
                AtlasAllocator::new(size2(*width as i32, remaining_height as i32));

            for (asset, allocated) in sorted.iter_mut() {
                if asset_paths_of_bucket.contains(asset.entry.asset_path.path()) {
                    let (pad_x, pad_y) = pad_for_image_asset(*asset);
                    let (w, h) = asset.rgba.dimensions();
                    let alloc_size = size2((w + 2 * pad_x) as i32, (h + 2 * pad_y) as i32);
                    if let Some(allocation) = remaining_size_allocator.allocate(alloc_size) {
                        let (mut x, mut y) = (
                            allocation.rectangle.min.x as u32,
                            allocation.rectangle.min.y as u32,
                        );
                        y += entries_height + pad_x;
                        x += pad_y;

                        // copy the image over and set allocated to true:
                        *allocated = true;
                        atlas.copy_from(&asset.rgba, x, y).unwrap();
                        let tile = TextureTile {
                            atlas: atlas_name(atlases.len()),
                            min: uvec2(x, y),
                            max: uvec2(x + w, y + h),
                        };
                        tiles.insert(asset.entry.asset_path.ident().to_owned(), tile);
                    }
                }
            }
        }

        atlases.push((atlas, TextureFlags::REPEAT_X));
    }

    for (height, entries) in rep_y_buckets.iter() {
        todo!("do the same as above for the rep_x_buckets. Was not really needed yet, so I saved the 5 min.");
    }

    use guillotiere::{size2, AtlasAllocator};
    let mut allocator = AtlasAllocator::new(size2(atlas_w as i32, atlas_h as i32));

    // let mut allocator = AtlasAllocator::new(Size::new(atlas_w as i32, atlas_h as i32));
    let mut atlas = RgbaImage::new(atlas_w, atlas_h);
    for (asset, allocated) in sorted.iter_mut() {
        if *allocated {
            continue;
        }
        let (pad_x, pad_y) = pad_for_image_asset(*asset);
        let (w, h) = asset.rgba.dimensions();

        if w > atlas_w || h > atlas_h {
            panic!("Only textures up to 1024x1024 supported! Just increase the allocator size if really necessary");
        }

        let alloc_size = size2((w + pad_x * 2) as i32, (h + pad_y * 2) as i32);
        let allocation = if let Some(alloc) = allocator.allocate(alloc_size) {
            alloc
        } else {
            // allocator is full, put in new allocator, flush atlas
            let last_atlas = std::mem::replace(&mut atlas, RgbaImage::new(atlas_w, atlas_h));
            atlases.push((last_atlas, TextureFlags::NO_REPEAT));
            allocator = AtlasAllocator::new(size2(atlas_w as i32, atlas_h as i32));
            allocator
                .allocate(alloc_size)
                .expect("The new allocator should be big enough now")
        };
        let (mut x, mut y) = (
            allocation.rectangle.min.x as u32,
            allocation.rectangle.min.y as u32,
        );
        y += pad_x;
        x += pad_y;
        // copy the image over and set allocated to true:
        *allocated = true;
        atlas.copy_from(&asset.rgba, x, y).unwrap();
        let tile = TextureTile {
            atlas: atlas_name(atlases.len()),
            min: uvec2(x, y),
            max: uvec2(x + w, y + h),
        };
        tiles.insert(asset.entry.asset_path.ident().to_owned(), tile);
    }
    atlases.push((atlas, TextureFlags::NO_REPEAT));

    (atlases, tiles)
}

fn atlas_name(i: usize) -> String {
    format!("atlas_{i}")
}

pub fn next_pow2_number(n: usize) -> usize {
    let mut e = 2;
    loop {
        if e >= n {
            return e;
        }
        e *= 2;
    }
}
