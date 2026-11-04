use crate::{err, storage, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Catalog {
    sheets: Vec<Sheet>,
}
#[derive(Deserialize)]
struct Sheet {
    id: String,
    file: String,
    width: u32,
    height: u32,
    frames: Vec<[u32; 4]>,
}
pub fn install(data: &Path) -> Result<()> {
    let config = data.join("prismlauncher.cfg");
    let configured = if config.exists() {
        storage::ini(&storage::read(&config, 1024 * 1024)?)
            .remove("IconsDir")
            .unwrap_or_else(|| "icons".into())
    } else {
        "icons".into()
    };
    let icons = data.join(configured);
    storage::directory(&icons)?;
    let icons = icons.join("graphite-v1");
    storage::directory(&icons)?;
    for (name, bytes) in [
        (
            "categories.png",
            include_bytes!("../../public/graphite-icons/categories.png").as_slice(),
        ),
        (
            "actions.png",
            include_bytes!("../../public/graphite-icons/actions.png").as_slice(),
        ),
        (
            "states.png",
            include_bytes!("../../public/graphite-icons/states.png").as_slice(),
        ),
    ] {
        let file = icons.join(format!("graphite-v1-atlas-{name}"));
        if !file.exists() {
            storage::write_new(&file, bytes)?;
        } else {
            storage::regular(&file)?;
        }
    }
    let catalog: Catalog =
        serde_json::from_str(include_str!("../../src/icon-catalog.json")).map_err(err)?;
    for sheet in catalog.sheets {
        for (index, [x, y, width, height]) in sheet.frames.into_iter().enumerate() {
            let file = icons.join(format!("graphite-v1-{}-{:02}.svg", sheet.id, index + 1));
            if file.exists() {
                storage::regular(&file)?;
                continue;
            }
            let svg = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="48" height="48" viewBox="{x} {y} {width} {height}"><defs><clipPath id="crop"><rect x="{x}" y="{y}" width="{width}" height="{height}"/></clipPath></defs><image clip-path="url(#crop)" xlink:href="graphite-v1-atlas-{}" width="{}" height="{}"/></svg>"##,
                sheet.file, sheet.width, sheet.height
            );
            storage::write_new(&file, svg.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn installs_all_icons_without_overwriting_custom_files() {
        let data = std::env::temp_dir().join(format!("graphite-icons-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&data).unwrap();
        super::install(&data).unwrap();
        let icons = data.join("icons/graphite-v1");
        assert_eq!(
            std::fs::read_dir(&icons)
                .unwrap()
                .filter_map(|f| f.ok())
                .filter(|f| f.path().extension().is_some_and(|v| v == "svg"))
                .count(),
            80
        );
        let file = icons.join("graphite-v1-categories-01.svg");
        std::fs::write(&file, "preserve-custom-content").unwrap();
        super::install(&data).unwrap();
        assert_eq!(
            std::fs::read_to_string(file).unwrap(),
            "preserve-custom-content"
        );
        assert!(
            std::fs::read_to_string(icons.join("graphite-v1-actions-32.svg"))
                .unwrap()
                .contains("graphite-v1-atlas-actions.png")
        );
        std::fs::write(
            data.join("prismlauncher.cfg"),
            "[General]\nIconsDir=custom icons\n",
        )
        .unwrap();
        super::install(&data).unwrap();
        assert!(data
            .join("custom icons/graphite-v1/graphite-v1-atlas-actions.png")
            .is_file());
        let actual = data.canonicalize().unwrap();
        assert_eq!(
            actual.parent(),
            Some(std::env::temp_dir().canonicalize().unwrap().as_path())
        );
        assert!(actual
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("graphite-icons-"));
        std::fs::remove_dir_all(actual).unwrap();
    }
}
