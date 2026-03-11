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
        padded_control(text::heading(fl!("output-devices")).width(Length::Fill))
    );

    for (index, name) in app.sound.sinks().iter().enumerate() {
        let active = app.sound.active_sink() == Some(index);
        let check_icon: Element<'_, Message> = if active {
            icon::from_name("object-select-symbolic").size(16).symbolic(true).into()
        } else {
            cosmic::iced::widget::Space::new().width(Length::Fixed(16.0)).into()
        };

        let dev_row = cosmic::widget::button::custom(
            row![
                icon::from_name("audio-speakers-symbolic").size(20).symbolic(true),
                text::body(name.to_string()).width(Length::Fill),
                check_icon,
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::SetDefaultSink(index))
        .class(cosmic::theme::Button::Standard);

        device_list = device_list.push(dev_row);
    }

    // Input section header
    device_list = device_list.push(
        padded_control(text::heading(fl!("input-devices")).width(Length::Fill))
    );

    for (index, name) in app.sound.sources().iter().enumerate() {
        let active = app.sound.active_source() == Some(index);
        let check_icon: Element<'_, Message> = if active {
            icon::from_name("object-select-symbolic").size(16).symbolic(true).into()
        } else {
            cosmic::iced::widget::Space::new().width(Length::Fixed(16.0)).into()
        };

        let dev_row = cosmic::widget::button::custom(
            row![
                icon::from_name("audio-input-microphone-symbolic").size(20).symbolic(true),
                text::body(name.to_string()).width(Length::Fill),
                check_icon,
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::SetDefaultSource(index))
        .class(cosmic::theme::Button::Standard);

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
