
use std::{sync::Arc, path::{Path, PathBuf}, env};
use ::icon_loader::{IconLoader, ThemeNameProvider::{GTK, KDE}, SearchPaths, Icon};
use resvg::{render, usvg::{Tree, Options}, tiny_skia::{Pixmap, Transform, PixmapPaint}};
use anyhow::{Result as Res, Context, anyhow};


pub fn find_icon(name: &str) -> Res<Arc<Icon>> {
    let mut loader = IconLoader::new();

    // round up search paths
    loader.set_search_paths(SearchPaths::System);

    let home_var = env::var("HOME").context("failed fetching env var HOME")?;
    let mut home_path = PathBuf::from(home_var);
    home_path.push(".local/share/icons");

    if !loader.search_paths().contains(&home_path) {
        let mut search_paths = loader.search_paths().into_owned();
        search_paths.push(home_path);
        loader.set_search_paths(search_paths);
    }

    // try with different themes
    let themes = [GTK, KDE, "Adwaita".into()];

    themes.into_iter().find_map(|provider| {
        loader.set_theme_name_provider(provider);
        loader.update_theme_name().ok()?;
        loader.load_icon(name)
    })
    .with_context(|| format!("icon '{name}' was not found"))
}


pub fn load_icon_as_size(name: &str, [w, h]: [u32; 2]) -> Res<Vec<u8>> {

    let icon = find_icon(name)?;

    let path = icon.file_for_size(w as u16).path(); // get best fitting size

    load_image_with_resize(path, |_| [w, h])
}


pub fn load_image_with_resize(path: impl AsRef<Path>, map_size: impl FnOnce([u32; 2]) -> [u32; 2]) -> Res<Vec<u8>> {

    let path = path.as_ref();

    match path.extension().and_then(|ext| ext.to_str()) {

        Some("png") => {

            let src = Pixmap::load_png(path).with_context(|| format!("failed loading icon from '{}'", path.display()))?;

            let [sw, sh] = [src.width(), src.height()];
            let [w, h] = map_size([sw, sh]); // possible resize

            if [sw, sh] == [w, h] { // if size matches
                Ok(src.take())
            }
            else {
                let mut pixmap = Pixmap::new(w, h).context("couldn't create pixmap")?;

                let trs = Transform::from_scale(w as f32 / sw as f32, h as f32 / sh as f32); // stretch to fill

                pixmap.draw_pixmap(0, 0, src.as_ref(), &PixmapPaint::default(), trs, None);

                Ok(pixmap.take())
            }
        },

        Some("svg") => {

            let svg_data = std::fs::read(path).with_context(|| format!("failed loading icon from '{}'", path.display()))?;

            let tree = Tree::from_data(&svg_data, &Options::default())?;

            let [w, h] = map_size([tree.size().width() as u32, tree.size().height() as u32]); // possible resize

            let mut pixmap = Pixmap::new(w, h).context("couldn't create pixmap")?;

            render(&tree, Transform::default(), &mut pixmap.as_mut());

            Ok(pixmap.take())
        },

        Some(ext) => Err(anyhow!("file type '{ext}' is not supported")),

        None => Err(anyhow!("unknown file type is not supported")),
    }
}
