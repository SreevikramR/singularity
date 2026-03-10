// SPDX-License-Identifier: GPL-3.0-only
//
// Bluetooth detail view: paired and available device list.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::views::detail_header::detail_header;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{icon, text};
use cosmic::applet::padded_control;

pub fn bluetooth_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("bluetooth-devices"),
        app.bluetooth_enabled,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleBluetooth(!app.bluetooth_enabled),
    );

    let mut device_list = column![].spacing(4).width(Length::Fill);

    if app.bluetooth_enabled {
        // Placeholder devices — will be replaced with BlueZ data
        let demo_devices = [
            ("AirPods Pro", "audio-headphones-symbolic", true),
            ("Magic Mouse", "input-mouse-symbolic", false),
            ("Keyboard K380", "input-keyboard-symbolic", true),
        ];

        for (name, icon_name, connected) in demo_devices {
            let status_text = if connected {
                fl!("connected")
            } else {
                fl!("available")
            };

            let dev_row = padded_control(
                row![
                    icon::from_name(icon_name).size(20).symbolic(true),
                    text::body(name.to_string()).width(Length::Fill),
                    text::caption(status_text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            );

            device_list = device_list.push(dev_row);
        }
    } else {
        device_list = device_list.push(
            padded_control(text::body(String::from("Bluetooth is disabled")).width(Length::Fill))
        );
    }

    let content = column![
        header,
        cosmic::widget::divider::horizontal::default(),
        scrollable(device_list).height(Length::Fill),
    ]
    .spacing(8)
    .padding(8);

    content.into()
}
