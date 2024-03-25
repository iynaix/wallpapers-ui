use clap::Parser;
use std::path::PathBuf;

use wallpaper_ui::{
    args::WallpaperUIArgs,
    cropper::AspectRatio,
    filename, filter_images,
    geometry::Geometry,
    wallpaper_dir,
    wallpapers::{WallInfo, WallpapersCsv},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub show_filelist: bool,
    pub show_faces: bool,
    pub preview_mode: PreviewMode,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_filelist: Default::default(),
            show_faces: Default::default(),
            preview_mode: PreviewMode::Source,
        }
    }
}

impl UiState {
    pub fn reset(&mut self) {
        self.show_filelist = false;
        self.show_faces = false;
        self.preview_mode = PreviewMode::Source;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewMode {
    Source,
    Default,
    Start,
    Center,
    End,
    Manual,
    None,
    /// stores the last mouseover geometry
    Candidate(Option<Geometry>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallpapers {
    pub files: Vec<PathBuf>,
    // the original wallinfo before any modifications
    pub source: WallInfo,
    pub current: WallInfo,
    pub index: usize,
    pub ratio: AspectRatio,
}

impl Default for Wallpapers {
    fn default() -> Self {
        Self {
            files: Vec::default(),
            source: WallInfo::default(),
            current: WallInfo::default(),
            index: Default::default(),
            ratio: AspectRatio(1440, 2560),
        }
    }
}

impl Wallpapers {
    pub fn from_args() -> Self {
        let args = WallpaperUIArgs::parse();
        let mut all_files = Vec::new();
        if let Some(paths) = args.paths {
            paths.iter().flat_map(std::fs::canonicalize).for_each(|p| {
                if p.is_file() {
                    all_files.push(p);
                } else {
                    all_files.extend(filter_images(&p));
                }
            });
        }

        if all_files.is_empty() {
            // defaults to wallpaper directory
            let wall_dir = wallpaper_dir();

            if !wall_dir.exists() {
                eprintln!("Wallpaper directory does not exist: {:?}", wall_dir);
                std::process::exit(1);
            }

            all_files.extend(filter_images(&wall_dir));
        }

        // order by reverse chronological order
        all_files.sort_by_key(|f| {
            f.metadata()
                .expect("could not get file metadata")
                .modified()
                .expect("could not get file mtime")
        });
        all_files.reverse();

        let wallpapers_csv = WallpapersCsv::new();
        let fname = filename(all_files.first().expect("no wallpapers found"));
        let loaded = wallpapers_csv
            .get(&fname)
            .expect("could not get wallpaper info");

        Self {
            files: all_files,
            source: loaded.clone(),
            current: loaded.clone(),
            ..Default::default()
        }
    }

    pub fn prev_wall(&mut self) {
        // loop back to the last wallpaper
        self.index = if self.index == 0 {
            self.files.len() - 1
        } else {
            self.index - 1
        };

        let wallpapers_csv = WallpapersCsv::new();
        let loaded = wallpapers_csv
            // bounds check is not necessary since the index is always valid
            .get(&filename(&self.files[self.index]))
            .expect("could not get wallpaper info");
        self.source = loaded.clone();
        self.current = loaded.clone();
    }

    pub fn next_wall(&mut self) {
        // loop back to the first wallpaper
        self.index = if self.index == self.files.len() - 1 {
            0
        } else {
            self.index + 1
        };

        let wallpapers_csv = WallpapersCsv::new();
        let loaded = wallpapers_csv
            // bounds check is not necessary since the index is always valid
            .get(&filename(&self.files[self.index]))
            .expect("could not get wallpaper info");
        self.source = loaded.clone();
        self.current = loaded.clone();
    }

    /// removes the current wallpaper from the list
    pub fn remove(&mut self) {
        let current_index = self.index;
        self.next_wall();
        self.files.remove(current_index);
        // current_index is unchanged after removal
        self.index = current_index;
    }

    pub fn set_from_filename(&mut self, fname: &str) {
        let wallpapers_csv = WallpapersCsv::new();
        let loaded = wallpapers_csv
            .get(fname)
            .expect("could not get wallpaper info")
            .clone();
        self.source = loaded.clone();
        self.current = loaded;
        self.index = self
            .files
            .iter()
            .position(|f| filename(f) == fname)
            .unwrap_or_else(|| panic!("could not find wallpaper: {}", fname));
    }

    /// gets geometry for current aspect ratio
    pub fn get_geometry(&self) -> Geometry {
        self.current.get_geometry(&self.ratio)
    }

    /// sets the geometry for current aspect ratio
    pub fn set_geometry(&mut self, geom: &Geometry) {
        self.current.set_geometry(&self.ratio, geom);
    }

    /// returns crop candidates for current ratio and image
    pub fn crop_candidates(&self) -> Vec<Geometry> {
        self.current.cropper().crop_candidates(&self.ratio)
    }
}
