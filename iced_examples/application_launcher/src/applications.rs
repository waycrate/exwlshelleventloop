use std::path::PathBuf;
use std::sync::LazyLock;

use crate::systemd;

use super::Message;
use freedesktop_desktop_entry::{self as fde, IconSource};
use iced::Pixels;
use iced::widget::{button, column, image, row, svg, text};
use iced::{Element, Length};

static LOCALE: LazyLock<Vec<String>> = LazyLock::new(fde::get_languages_from_env);

static DEFAULT_ICON: &[u8] = include_bytes!("../misc/text-plain.svg");
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct App {
    id: String,
    name: String,
    cmds: Vec<String>,
    description: String,
    pub categrades: Option<Vec<String>>,
    pub actions: Option<Vec<String>>,
    icon: Option<PathBuf>,
}

impl App {
    pub async fn launch(&self) {
        if let Err(err) = systemd::launch(&self.id, &self.cmds, &self.description).await {
            tracing::error!("{err}");
        };
    }

    pub fn title(&self) -> &str {
        &self.name
    }

    fn icon(&self) -> Element<'_, Message> {
        match &self.icon {
            Some(path) => {
                if path.extension().is_some_and(|extension| extension == "png") {
                    image(image::Handle::from_path(path))
                        .width(Length::Fixed(80.))
                        .height(Length::Fixed(80.))
                        .into()
                } else {
                    svg(svg::Handle::from_path(path))
                        .width(Length::Fixed(80.))
                        .height(Length::Fixed(80.))
                        .into()
                }
            }
            None => svg(svg::Handle::from_memory(DEFAULT_ICON))
                .width(Length::Fixed(80.))
                .height(Length::Fixed(80.))
                .into(),
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn view(&'_ self, index: usize, selected: bool) -> Element<'_, Message> {
        button(
            row![
                self.icon(),
                column![
                    text(self.title()).size(Pixels::from(20)),
                    text(self.description()).size(Pixels::from(10))
                ]
                .spacing(4)
            ]
            .spacing(10),
        )
        .on_press(Message::Launch(index))
        .width(Length::Fill)
        .height(Length::Fixed(85.))
        .style(if selected {
            button::primary
        } else {
            button::secondary
        })
        .into()
    }
}

static ICONS_SIZE: &[&str] = &["256x256", "128x128", "64x64", "48x48", "32x32", "16x16"];

static THEMES_LIST: &[&str] = &["breeze", "Adwaita"];

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
    let top_icon_path = xdg::BaseDirectories::with_prefix("icons");

    // NOTE: shit application icon place
    if let Some(iconpath) = top_icon_path.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }

    // NOTE: shit application icon place
    if let Some(iconpath) = top_icon_path.find_data_file(format!("{iconname}.png")) {
        return Some(iconpath);
    }

    let scalable_icon_path = xdg::BaseDirectories::with_prefix("icons/hicolor/scalable/apps");
    if let Some(iconpath) = scalable_icon_path.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }
    for prefix in ICONS_SIZE {
        let iconpath = xdg::BaseDirectories::with_prefix(format!("icons/hicolor/{prefix}/apps"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.png")) {
            return Some(iconpath);
        }
    }
    let pixmappath = xdg::BaseDirectories::with_prefix("pixmaps");
    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }
    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.png")) {
        return Some(iconpath);
    }
    for themes in THEMES_LIST {
        let iconpath = xdg::BaseDirectories::with_prefix(format!("icons/{themes}/apps/48"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.svg")) {
            return Some(iconpath);
        }
        let iconpath = xdg::BaseDirectories::with_prefix(format!("icons/{themes}/apps/64"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.svg")) {
            return Some(iconpath);
        }
    }
    None
}

fn get_icon_path(iconname: fde::IconSource) -> Option<PathBuf> {
    match iconname {
        IconSource::Name(name) => get_icon_path_from_xdgicon(name.as_str()),
        IconSource::Path(path) => Some(path),
    }
}

pub fn all_apps() -> Vec<App> {
    let desktop_entries = fde::desktop_entries(&LOCALE);
    desktop_entries
        .iter()
        .filter(|entry| !entry.no_display() && !entry.hidden())
        .flat_map(|entry| {
            let cmds = entry.parse_exec().ok()?;
            let id = entry.id().to_string();
            let name = entry.name(&LOCALE).map(|n| n.to_string())?;
            let description = entry
                .comment(&LOCALE)
                .map(|c| c.to_string())
                .unwrap_or(format!("Run {name}"));
            let categrades: Option<Vec<String>> = entry
                .categories()
                .map(|c| c.iter().map(|i| i.to_string()).collect::<Vec<String>>());
            let actions: Option<Vec<String>> = entry
                .actions()
                .map(|c| c.iter().map(|i| i.to_string()).collect());
            let icon = get_icon_path(fde::IconSource::from_unknown(
                entry.icon().unwrap_or_default(),
            ));
            Some(App {
                id,
                name,
                description,
                cmds,
                categrades,
                actions,
                icon,
            })
        })
        .collect()
}
