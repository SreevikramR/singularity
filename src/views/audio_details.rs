// SPDX-License-Identifier: GPL-3.0-only
//
// Audio detail view: output and input device selection with volume sliders and revealer.

use crate::app::{AppModel, IsOpen, Message};
use crate::fl;
use crate::views::detail_header::detail_header;
use cosmic::applet::padded_control;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, slider, text};

fn revealer(
    open: bool,
    title: String,
    selected: String,
    devices: &[String],
    toggle: Message,
    mut change: impl FnMut(usize) -> Message + 'static,
) -> cosmic::iced::widget::Column<'static, Message, cosmic::Theme, cosmic::iced::Renderer> {
    if open {
        devices
            .iter()
            .cloned()
            .enumerate()
            .fold(
                column![revealer_head(open, title, selected, toggle)].width(Length::Fill),
                |col, (id, name)| {
                    col.push(
                        button::custom(text::body(name))
                            .on_press(change(id))
                            .width(Length::Fill)
                            .padding([8, 16])
                            .class(cosmic::theme::Button::Standard),
                    )
                },
            )
    } else {
        column![revealer_head(open, title, selected, toggle)]
    }
}

fn revealer_head(
    _open: bool,
    title: String,
    selected: String,
    toggle: Message,
) -> cosmic::widget::Button<'static, Message> {
    button::custom(column![
        text::body(title).width(Length::Fill),
        text::caption(selected),
    ])
    .on_press(toggle)
    .class(cosmic::theme::Button::Standard)
}

pub fn audio_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("audio-devices"),
        !app.global_mute,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleGlobalMute(!app.global_mute),
    );

    let output_icon = if app.volume == 0 || app.global_mute {
        "audio-volume-muted-symbolic"
    } else if app.volume < 33 {
        "audio-volume-low-symbolic"
    } else if app.volume < 66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    };

    let input_icon = if app.sound.source_volume == 0 || app.sound.source_mute {
        "microphone-sensitivity-muted-symbolic"
    } else if app.sound.source_volume < 33 {
        "microphone-sensitivity-low-symbolic"
    } else if app.sound.source_volume < 66 {
        "microphone-sensitivity-medium-symbolic"
    } else {
        "microphone-sensitivity-high-symbolic"
    };

    let sink = app
        .sound
        .active_sink()
        .and_then(|pos| app.sound.sinks().get(pos));

    let source = app
        .sound
        .active_source()
        .and_then(|pos| app.sound.sources().get(pos));

    let mut device_list = column![].spacing(8).width(Length::Fill);

    // Default icon padding in cosmic
    let icon_padding = 4;

    // Output section
    let output_slider = slider(
        0..=app.max_sink_volume,
        app.volume,
        Message::SetVolume,
    )
    .width(Length::Fill);

    let output_row = padded_control(
        row![
            button::icon(icon::from_name(output_icon).size(20).symbolic(true))
                .padding(icon_padding)
                .width(Length::Fixed(28.0))
                .on_press(Message::ToggleGlobalMute(!app.global_mute)),
            output_slider,
            cosmic::iced::widget::container(text(&app.sound.sink_volume_text).size(14))
                .width(Length::Fixed(40.0))
                .align_x(Alignment::End)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let output_revealer = revealer(
        app.is_open == IsOpen::Output,
        "Output Devices".to_string(),
        match sink {
            Some(sink) => sink.to_owned(),
            None => "No output device".to_string(),
        },
        app.sound.sinks(),
        Message::OutputToggle,
        Message::SetDefaultSink,
    );

    device_list = device_list.push(output_row).push(output_revealer);

    // Separator
    device_list = device_list.push(padded_control(cosmic::widget::divider::horizontal::default()));

    // Input section
    let input_slider_val = app.sound.source_volume;
    let input_slider = slider(
        0..=app.max_source_volume,
        input_slider_val,
        Message::SetSourceVolume,
    )
    .width(Length::Fill);

    let input_row = padded_control(
        row![
            button::icon(icon::from_name(input_icon).size(20).symbolic(true))
                .padding(icon_padding)
                .width(Length::Fixed(28.0))
                .on_press(Message::ToggleSourceMute),
            input_slider,
            cosmic::iced::widget::container(text(&app.sound.source_volume_text).size(14))
                .width(Length::Fixed(40.0))
                .align_x(Alignment::End)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let input_revealer = revealer(
        app.is_open == IsOpen::Input,
        "Input Devices".to_string(),
        match source {
            Some(source) => source.to_owned(),
            None => "No input device".to_string(),
        },
        app.sound.sources(),
        Message::InputToggle,
        Message::SetDefaultSource,
    );

    device_list = device_list.push(input_row).push(input_revealer);

    let content = column![
        header,
        cosmic::widget::divider::horizontal::default(),
        scrollable(device_list.padding(8)).height(Length::Fill),
    ]
    .spacing(4);

    content.into()
}
