use std::path::PathBuf;
use std::sync::LazyLock;

use crate::{Message, systemd};
use freedesktop_desktop_entry::{self as fde, IconSource};
use iced::border::Radius;
use iced::widget::{button, image, svg};
use iced::{Background, Border, Element, Length, Shadow, Vector};

static LOCALE: LazyLock<Vec<String>> = LazyLock::new(fde::get_languages_from_env);
static DEFAULT_ICON: &[u8] = include_bytes!("../misc/text-plain.svg");

#[derive(Debug, Clone)]
pub struct App {
    id: String,
    cmds: Vec<String>,
    description: String,
    icon: Option<PathBuf>,
}

impl App {
    pub async fn launch(&self) {
        if let Err(err) = systemd::launch(&self.id, &self.cmds, &self.description).await {
            tracing::error!("{err}");
        };
    }

    fn icon(&self) -> Element<'_, Message> {
        match &self.icon {
            Some(path) => {
                if path.extension().is_some_and(|ext| ext == "png") {
                    image(image::Handle::from_path(path))
                        .width(Length::Fixed(80.))
                        .height(Length::Fixed(80.))
                        .into()
                } else {
                    svg(svg::Handle::from_path(path))
                        .width(Length::Fixed(40.))
                        .height(Length::Fixed(40.))
                        .into()
                }
            }
            None => svg(svg::Handle::from_memory(DEFAULT_ICON))
                .width(Length::Fixed(40.))
                .height(Length::Fixed(40.))
                .into(),
        }
    }

    pub fn view(&self, index: usize, _selected: bool) -> Element<'_, Message> {
        button(self.icon())
            .on_press(Message::Launch(index))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme, _status| button::Style {
                background: Some(Background::Color(iced::Color::from_rgba(
                    0.188, 0.192, 0.188, 0.65,
                ))),
                text_color: iced::Color::WHITE,
                border: Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: Radius {
                        top_left: 0.0,
                        top_right: 0.0,
                        bottom_right: 0.0,
                        bottom_left: 0.0,
                    },
                },
                shadow: Shadow {
                    color: iced::Color::TRANSPARENT,
                    offset: Vector { x: 0.0, y: 2.0 },
                    blur_radius: 5.0,
                },
                snap: true,
            })
            .into()
    }
}

static ICONS_SIZE: &[&str] = &["256x256", "256x256"];

static THEMES_LIST: &[&str] = &["yaru", "breeze", "Adwaita"];

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
        .take(10)
        .flat_map(|entry| {
            let cmds = entry.parse_exec().ok()?;
            let id = entry.id().to_string();
            let name = entry.name(&LOCALE).map(|n| n.to_string())?;
            let description = entry
                .comment(&LOCALE)
                .map(|c| c.to_string())
                .unwrap_or(format!("Run {name}"));
            let icon = get_icon_path(fde::IconSource::from_unknown(
                entry.icon().unwrap_or_default(),
            ));
            Some(App {
                id,
                description,
                cmds,
                icon,
            })
        })
        .collect()
}
