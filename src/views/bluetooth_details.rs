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
        let mut devices_sorted: Vec<_> = app.bt_devices.iter().collect();
        // Sort by connected first, then name
        devices_sorted.sort_by(|a, b| {
            let a_conn = a.1.is_connected();
            let b_conn = b.1.is_connected();
            if a_conn != b_conn {
                b_conn.cmp(&a_conn)
            } else {
                a.1.alias_or_addr().cmp(b.1.alias_or_addr())
            }
        });

        for (path, device) in devices_sorted {
            let connected = device.is_connected();
            let status_text = if connected {
                fl!("connected")
            } else {
                fl!("available")
            };

            let icon_name = if device.icon.is_empty() {
                "bluetooth-active-symbolic"
            } else {
                &device.icon
            };

            let dev_row = cosmic::widget::button::custom(
                row![
                    icon::from_name(icon_name).size(20).symbolic(true),
                    text::body(device.alias_or_addr().to_string()).width(Length::Fill),
                    text::caption(status_text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .padding(8)
            .on_press(if connected {
                Message::DisconnectBluetoothDevice(path.clone())
            } else {
                Message::ConnectBluetoothDevice(path.clone())
            })
            .class(cosmic::theme::Button::Standard);

            device_list = device_list.push(dev_row);
        }
    } else {
        device_list = device_list.push(
            padded_control(text::body(fl!("bluetooth-disabled")).width(Length::Fill))
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
