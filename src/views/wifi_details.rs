// SPDX-License-Identifier: GPL-3.0-only
//
// Wi-Fi detail view: scrollable list of available networks and password entry.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::network::NewConnectionState;
use crate::views::detail_header::detail_header;
use cosmic::applet::padded_control;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, text, text_input};
use cosmic_settings_network_manager_subscription::available_wifi::NetworkType;
use secure_string::SecureString;
use std::sync::Arc;

pub fn wifi_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("wifi-networks"),
        app.nm_state.wifi_enabled,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleWifi(!app.nm_state.wifi_enabled),
    );

    let mut network_list = column![].spacing(8).width(Length::Fill);

    if !app.nm_state.wifi_enabled {
        network_list = network_list.push(
            cosmic::widget::container(padded_control(text::body(fl!("wifi-disabled"))))
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .padding(16),
        );
    } else if let Some(new_conn_state) = app.new_connection.as_ref() {
        match new_conn_state {
            NewConnectionState::EnterPassword {
                access_point,
                description,
                identity,
                password,
                password_hidden,
            } => {
                let id = row![
                    icon::from_name("network-wireless-acquiring-symbolic")
                        .size(24)
                        .symbolic(true),
                    text::body(access_point.ssid.as_ref()),
                ]
                .align_y(Alignment::Center)
                .spacing(12);

                let is_enterprise = matches!(access_point.network_type, NetworkType::EAP);
                let enter_password_col = column![]
                    .push_maybe(is_enterprise.then(|| text::body("Identity")))
                    .push_maybe(is_enterprise.then(|| {
                        text_input::text_input("", identity).on_input(Message::IdentityUpdate)
                    }))
                    .push(text::body("Enter Password"))
                    .push_maybe(description.as_ref().map(|d| text::body(d.clone())))
                    .push(
                        text_input::secure_input(
                            "",
                            password.unsecure(),
                            Some(Message::TogglePasswordVisibility),
                            *password_hidden,
                        )
                        .on_input(|s| Message::PasswordUpdate(SecureString::from(s)))
                        .on_paste(|s| Message::PasswordUpdate(SecureString::from(s)))
                        .on_submit(|_| Message::ConnectWithPassword),
                    )
                    .push(
                        row![
                            button::standard(fl!("cancel")).on_press(Message::CancelNewConnection),
                            button::suggested(fl!("connect")).on_press(Message::ConnectWithPassword)
                        ]
                        .spacing(24),
                    );

                network_list = network_list
                    .push(padded_control(id))
                    .push(padded_control(enter_password_col.spacing(8)));
            }
            NewConnectionState::Waiting(access_point) => {
                let connecting = padded_control(
                    row![
                        icon::from_name("network-wireless-acquiring-symbolic")
                            .size(24)
                            .symbolic(true),
                        text::body(access_point.ssid.as_ref()).width(Length::Fill),
                        icon::from_name("process-working-symbolic")
                            .size(24)
                            .symbolic(true),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(12),
                );
                network_list = network_list.push(connecting);
            }
            NewConnectionState::Failure(access_point) => {
                let id = padded_control(
                    row![
                        icon::from_name("network-wireless-error-symbolic")
                            .size(24)
                            .symbolic(true),
                        text::body(access_point.ssid.as_ref()),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(12),
                );
                let error_col = padded_control(
                    column![
                        text("Unable to connect"),
                        text("Check Wi-Fi connection").size(14),
                        row![
                            button::standard(fl!("cancel")).on_press(Message::CancelNewConnection),
                            button::suggested(fl!("connect")).on_press(
                                Message::SelectWirelessAccessPoint(access_point.clone())
                            )
                        ]
                        .spacing(24)
                    ]
                    .spacing(16),
                );
                network_list = network_list.push(id).push(error_col);
            }
        }
    } else {
        let mut network_info = Vec::new();
        let active_conns = &app.nm_state.active_conns;
        let active_ssids: std::collections::HashSet<_> = active_conns
            .iter()
            .filter_map(|conn| {
                if let cosmic_settings_network_manager_subscription::ActiveConnectionInfo::WiFi {
                    name,
                    ..
                } = conn
                {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        // Include both known access points (so we can connect/disconnect) and discovered wireless points
        let mut all_aps = app.nm_state.wireless_access_points.clone();
        for known in &app.nm_state.known_access_points {
            if !all_aps.iter().any(|ap| ap.ssid == known.ssid) {
                // Not strictly accurate for strength, but includes offline known networks
                let mut fake_ap = known.clone();
                fake_ap.strength = 0;
                all_aps.push(fake_ap);
            }
        }

        for ap in all_aps {
            let ssid = ap.ssid.to_string();
            if ssid.is_empty() {
                continue;
            }
            let is_connected = active_ssids.contains(&ssid);
            network_info.push((ssid, is_connected, ap));
        }

        network_info.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.strength.cmp(&a.2.strength)));
        network_info.dedup_by(|a, b| a.0 == b.0);

        for (name, connected, ap) in network_info {
            let signal_icon = if ap.strength > 75 {
                "network-wireless-signal-excellent-symbolic"
            } else if ap.strength > 50 {
                "network-wireless-signal-good-symbolic"
            } else if ap.strength > 25 {
                "network-wireless-signal-ok-symbolic"
            } else {
                "network-wireless-signal-weak-symbolic"
            };

            let mut right_side = vec![];
            if connected {
                right_side.push(text::body(fl!("connected")).into());
                right_side.push(
                    icon::from_name("object-select-symbolic")
                        .size(16)
                        .symbolic(true)
                        .into(),
                );
            } else if matches!(ap.network_type, NetworkType::Open) {
                // show open indicator
            } else {
                right_side.push(
                    icon::from_name("network-wireless-encrypted-symbolic")
                        .size(16)
                        .symbolic(true)
                        .into(),
                );
            }

            let net_row = cosmic::widget::button::custom(
                row![
                    icon::from_name(signal_icon).size(24).symbolic(true),
                    text::body(name.clone()).width(Length::Fill),
                    cosmic::iced::widget::Row::with_children(right_side)
                        .spacing(8)
                        .align_y(Alignment::Center),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .padding(12)
            .class(cosmic::theme::Button::Standard);

            let net_row = if connected {
                net_row.on_press(Message::Disconnect(Arc::from(name.as_str()), ap.hw_address))
            } else {
                net_row.on_press(Message::SelectWirelessAccessPoint(ap.clone()))
            };

            network_list = network_list.push(net_row);
        }
    }

    let content = column![
        header,
        cosmic::widget::divider::horizontal::default(),
        scrollable(network_list.padding(8)).height(Length::Fill),
    ]
    .spacing(4);

    content.into()
}
