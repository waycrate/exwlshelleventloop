use std::path::PathBuf;
use std::str::FromStr;

use gio::{AppLaunchContext, DesktopAppInfo};

use gio::prelude::*;
use iced::Pixels;
use iced::widget::{button, column, image, row, svg, text};
use iced::{Element, Length};

use crate::Message;

static DEFAULT_ICON: &[u8] = include_bytes!("../misc/text-plain.svg");

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct App {
    appinfo: DesktopAppInfo,
    name: String,
    descriptions: Option<gio::glib::GString>,
    pub categrades: Option<Vec<String>>,
    pub actions: Option<Vec<gio::glib::GString>>,
    icon: Option<PathBuf>,
}

impl App {
    pub fn launch(&self) {
        if let Err(err) = self.appinfo.launch(&[], AppLaunchContext::NONE) {
            println!("{}", err);
        };
    }

    pub fn title(&self) -> &str {
        &self.name
    }

    fn icon(&self) -> Element<Message> {
        match &self.icon {
            Some(path) => {
                if path
                    .as_os_str()
                    .to_str()
                    .is_some_and(|pathname| pathname.ends_with("png"))
                {
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
        match &self.descriptions {
            None => "",
            Some(description) => description,
        }
    }

    pub fn view(&self, index: usize, selected: bool) -> Element<Message> {
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
        .style(move |theme, status| {
            if selected {
                button::primary(theme, status)
            } else {
                button::secondary(theme, status)
            }
        })
        .into()
    }
}

static ICONS_SIZE: &[&str] = &["256x256", "128x128"];

static THEMES_LIST: &[&str] = &["breeze", "Adwaita"];

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
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

fn get_icon_path(iconname: &str) -> Option<PathBuf> {
    if iconname.contains('/') {
        PathBuf::from_str(iconname).ok()
    } else {
        get_icon_path_from_xdgicon(iconname)
    }
}

pub fn all_apps() -> Vec<App> {
    let re = regex::Regex::new(r"([a-zA-Z]+);").unwrap();
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show() && app.downcast_ref::<gio::DesktopAppInfo>().is_some())
        .map(|app| app.clone().downcast::<gio::DesktopAppInfo>().unwrap())
        .map(|app| App {
            appinfo: app.clone(),
            name: app.name().to_string(),
            descriptions: app.description(),
            categrades: match app.categories() {
                None => None,
                Some(categrades) => {
                    let tomatch = categrades.to_string();
                    let tips = re
                        .captures_iter(&tomatch)
                        .map(|unit| unit.get(1).unwrap().as_str().to_string())
                        .collect();
                    Some(tips)
                }
            },
            actions: {
                let actions = app.list_actions();
                if actions.is_empty() {
                    None
                } else {
                    Some(actions)
                }
            },
            icon: match &app.icon() {
                None => None,
                Some(icon) => {
                    let iconname = gio::prelude::IconExt::to_string(icon).unwrap();
                    get_icon_path(iconname.as_str())
                }
            },
        })
        .collect()
}
