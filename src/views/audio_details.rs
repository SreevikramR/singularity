// SPDX-License-Identifier: GPL-3.0-only
//
// Audio detail view: output and input device selection.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::views::detail_header::detail_header;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{icon, text};
use cosmic::applet::padded_control;

pub fn audio_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("audio-devices"),
        !app.global_mute,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleGlobalMute(!app.global_mute),
    );

    let mut device_list = column![].spacing(4).width(Length::Fill);

    // Output section header
    device_list = device_list.push(
        padded_control(text::heading(String::from("Output")).width(Length::Fill))
    );

    // Placeholder output devices
    let output_devices = [
        ("Built-in Speakers", true),
        ("HDMI Audio", false),
        ("Bluetooth Headphones", false),
    ];

    for (name, active) in output_devices {
        let check_icon = if active {
            "object-select-symbolic"
        } else {
            "content-loading-symbolic"
        };

        let dev_row = padded_control(
            row![
                icon::from_name("audio-speakers-symbolic").size(20).symbolic(true),
                text::body(name.to_string()).width(Length::Fill),
                if active {
                    icon::from_name(check_icon).size(16).symbolic(true)
                } else {
                    icon::from_name("").size(16).symbolic(true)
                },
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );

        device_list = device_list.push(dev_row);
    }

    // Input section header
    device_list = device_list.push(
        padded_control(text::heading(String::from("Input")).width(Length::Fill))
    );

    let input_devices = [
        ("Built-in Microphone", true),
        ("USB Microphone", false),
    ];

    for (name, active) in input_devices {
        let dev_row = padded_control(
            row![
                icon::from_name("audio-input-microphone-symbolic").size(20).symbolic(true),
                text::body(name.to_string()).width(Length::Fill),
                if active {
                    icon::from_name("object-select-symbolic").size(16).symbolic(true)
                } else {
                    icon::from_name("").size(16).symbolic(true)
                },
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );

        device_list = device_list.push(dev_row);
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
