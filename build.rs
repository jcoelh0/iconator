use fst::Map;
use std::path::Path;
use std::{env, fs};

include!("./icons/lookup_table.rs");

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    build_lookup_tables(out_dir);
    build_svg_assets(out_dir);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icons");
}

fn build_lookup_tables(out_dir: &Path) {
    let ext_icons_path = out_dir.join("ext_icons.bin");
    let ext_icons = Map::from_iter(EXT_ICONS).unwrap();
    fs::write(ext_icons_path, ext_icons.into_fst().as_bytes()).unwrap();

    let filename_icons_path = out_dir.join("filename_icons.bin");
    let filename_icons = Map::from_iter(FILENAME_ICONS).unwrap();
    fs::write(filename_icons_path, filename_icons.into_fst().as_bytes()).unwrap();

    let folder_icons_path = out_dir.join("folder_icons.bin");
    let folder_icons = Map::from_iter(FOLDER_ICONS).unwrap();
    fs::write(folder_icons_path, folder_icons.into_fst().as_bytes()).unwrap();
}

fn build_svg_assets(out_dir: &Path) {
    // Pack SVG assets into icons_data.bin + an 8-byte-per-entry table of contents in icons_index.bin
    let mut icon_files: Vec<(u64, String)> = fs::read_dir("icons")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().into_string().ok()?;
            if file_name.ends_with(".svg") {
                let id = file_name.trim_end_matches(".svg").parse::<u64>().ok()?;
                Some((id, file_name))
            } else {
                None
            }
        })
        .collect();

    icon_files.sort_by_key(|(id, _)| *id);

    let max_id = icon_files.iter().map(|(id, _)| *id).max().unwrap_or(0) as usize;

    let mut index_bytes = vec![0u8; (max_id + 1) * 8];
    let mut data_bytes = Vec::new();

    for (id, file_name) in icon_files {
        let path = Path::new("icons").join(file_name);
        let contents = fs::read(path).unwrap();
        let offset = data_bytes.len() as u32;
        let len = contents.len() as u32;

        let entry_offset = id as usize * 8;
        index_bytes[entry_offset..entry_offset + 4].copy_from_slice(&offset.to_le_bytes());
        index_bytes[entry_offset + 4..entry_offset + 8].copy_from_slice(&len.to_le_bytes());

        data_bytes.extend_from_slice(&contents);
    }

    let icon_data_path = out_dir.join("icons_data.bin");
    fs::write(icon_data_path, data_bytes).unwrap();

    let icon_index_path = out_dir.join("icons_index.bin");
    fs::write(icon_index_path, index_bytes).unwrap();
}
