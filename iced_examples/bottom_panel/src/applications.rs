use std::path::PathBuf;
use std::str::FromStr;

use gio::{AppLaunchContext, DesktopAppInfo};

use crate::Message;
use gio::prelude::*;
use iced::border::Radius;
use iced::widget::{button, image, row, svg};
use iced::Background::Color;
use iced::{Background, Border, Element, Length, Shadow, Theme, Vector};

static DEFAULT_ICON: &[u8] = include_bytes!("../misc/text-plain.svg");

#[derive(Debug, Clone)]
pub struct App {
    app_info: DesktopAppInfo,
    icon: Option<PathBuf>,
}

impl App {
    pub fn launch(&self) {
        self.app_info.launch(&[], AppLaunchContext::NONE).unwrap()
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

    pub fn view(&self, index: usize, selected: bool) -> Element<Message> {
        button(row![self.icon(),].spacing(10))
            .on_press(Message::Launch(index))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |theme, status| button::Style {
                background: Some(Background::Color(iced::Color::from_rgba(
                    0.188, 0.192, 0.188, 0.65
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
            })
            .into()
    }
}

static ICONS_SIZE: &[&str] = &["256x256", "256x256"];

static THEMES_LIST: &[&str] = &["yaru"];

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
    let scalable_icon_path =
        xdg::BaseDirectories::with_prefix("icons/hicolor/scalable/apps").unwrap();
    if let Some(iconpath) = scalable_icon_path.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }
    for prefix in ICONS_SIZE {
        let iconpath =
            xdg::BaseDirectories::with_prefix(format!("icons/hicolor/{prefix}/apps")).unwrap();
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.png")) {
            return Some(iconpath);
        }
    }
    let pixmappath = xdg::BaseDirectories::with_prefix("pixmaps").unwrap();
    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }
    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.png")) {
        return Some(iconpath);
    }
    for themes in THEMES_LIST {
        let iconpath =
            xdg::BaseDirectories::with_prefix(format!("icons/{themes}/apps/48")).unwrap();
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.svg")) {
            return Some(iconpath);
        }
        let iconpath =
            xdg::BaseDirectories::with_prefix(format!("icons/{themes}/apps/64")).unwrap();
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
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show() && app.downcast_ref::<DesktopAppInfo>().is_some())
        .map(|app| app.clone().downcast::<DesktopAppInfo>().unwrap())
        .take(10)
        .map(|app| App {
            app_info: app.clone(),
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
