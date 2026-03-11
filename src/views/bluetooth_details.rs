// SPDX-License-Identifier: GPL-3.0-only
//
// Bluetooth detail view: paired and available device list.

use crate::app::{AppModel, Message};
use crate::bluetooth::{BluerDeviceStatus, BluerRequest};
use crate::fl;
use crate::views::detail_header::detail_header;
use cosmic::iced::widget::{column, row, scrollable, Space};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, text};
use cosmic::applet::padded_control;

pub fn bluetooth_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("bluetooth-devices"),
        app.bluer_state.bluetooth_enabled,
        Message::Navigate(crate::app::AppView::Main),
        Message::BluetoothRequest(BluerRequest::SetBluetoothEnabled(!app.bluer_state.bluetooth_enabled)),
    );

    let mut device_list = column![].spacing(4).width(Length::Fill);

    // Agent request overlays
    if let Some((device, code, _)) = &app.request_confirmation {
        let confirm_view = column![
            text::title3(fl!("bluetooth-pairing-request")),
            text::body(fl!("bluetooth-pairing-request-body", device_name = device.name.as_str())),
            text::title1(code.clone()).width(Length::Fill).align_x(Alignment::Center),
            row![
                button::standard(fl!("cancel"))
                    .on_press(Message::CancelPairing)
                    .width(Length::Fill),
                button::suggested(fl!("confirm"))
                    .on_press(Message::ConfirmPairing)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .width(Length::Fill)
        ]
        .spacing(16)
        .padding(16);

        return column![header, cosmic::widget::divider::horizontal::default(), confirm_view]
            .spacing(8)
            .padding(8)
            .into();
    }

    if app.bluer_state.bluetooth_enabled {
        let devices_sorted = app.bluer_state.devices.clone();
        
        // Split and filter devices
        let (paired, available): (Vec<_>, Vec<_>) = devices_sorted.into_iter().partition(|d| d.is_paired);
        
        if paired.is_empty() && available.is_empty() {
             device_list = device_list.push(
                padded_control(text::body(fl!("bluetooth-no-devices-found")).width(Length::Fill))
             );
        } else {
            // Paired Devices Section
            if !paired.is_empty() {
                device_list = device_list.push(
                    text::body(fl!("bluetooth-paired-devices"))
                );
                
                for device in paired {
                    device_list = device_list.push(device_row(&device));
                }
            }

            // Available Devices Section
            if !available.is_empty() {
                device_list = device_list.push(Space::new().height(16));
                
                device_list = device_list.push(
                    row![
                        text::body(fl!("bluetooth-available-devices")),
                        Space::new().width(Length::Fill),
                        cosmic::widget::toggler(app.show_visible_devices)
                            .on_toggle(Message::ToggleVisibleDevices)
                    ].align_y(Alignment::Center).width(Length::Shrink)
                );

                if app.show_visible_devices {
                    for device in available.into_iter().filter(|d: &crate::bluetooth::BluerDevice| d.is_known_device_type() || d.has_name()) {
                        device_list = device_list.push(device_row(&device));
                    }
                }
            }
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

fn device_row(device: &crate::bluetooth::BluerDevice) -> Element<'static, Message> {
    let mut status_text = match device.status {
        BluerDeviceStatus::Connected => fl!("connected"),
        BluerDeviceStatus::Connecting => fl!("connecting"),
        BluerDeviceStatus::Disconnected => fl!("disconnected"),
        BluerDeviceStatus::Disconnecting => fl!("disconnecting"),
        BluerDeviceStatus::Paired => fl!("not-connected"),
        BluerDeviceStatus::Pairing => fl!("pairing"),
    };
    
    if !device.is_paired {
        status_text = fl!("available");
    }

    let is_connected = matches!(device.status, BluerDeviceStatus::Connected);
    let mut row_content = row![
        icon::from_name(device.icon).size(20).symbolic(true),
        column![
            text::body(device.name.clone()),
            text::caption(status_text),
        ]
        .width(Length::Fill)
        .spacing(2),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    if let Some(bat) = device.battery_percent {
        row_content = row_content.push(
            row![
                text::caption(format!("{}%", bat)),
                icon::from_name("battery-symbolic").size(16).symbolic(true),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        );
    }

    let action = if device.is_paired {
        if is_connected {
            Message::BluetoothRequest(BluerRequest::DisconnectDevice(device.address))
        } else {
            Message::BluetoothRequest(BluerRequest::ConnectDevice(device.address))
        }
    } else {
        Message::BluetoothRequest(BluerRequest::PairDevice(device.address))
    };

    cosmic::widget::button::custom(row_content)
        .width(Length::Fill)
        .padding(8)
        .on_press(action)
        .class(cosmic::theme::Button::Standard)
        .into()
}
