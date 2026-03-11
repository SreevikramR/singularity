// SPDX-License-Identifier: GPL-3.0-only
//
// Wi-Fi detail view: scrollable list of available networks.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::views::detail_header::detail_header;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{icon, text};
use cosmic::applet::padded_control;

pub fn wifi_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("wifi-networks"),
        app.nm_state.wifi_enabled,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleWifi(!app.nm_state.wifi_enabled),
    );

    let mut network_list = column![].spacing(4).width(Length::Fill);

    if app.nm_state.wifi_enabled {
        let mut network_info: Vec<(String, bool, u8)> = Vec::new();
        let active_ssids: std::collections::HashSet<_> = app.nm_state.active_conns.iter().filter_map(|conn| {
            if let cosmic_settings_network_manager_subscription::ActiveConnectionInfo::WiFi { name, .. } = conn {
                Some(name.clone())
            } else {
                None
            }
        }).collect();

        for ap in &app.nm_state.wireless_access_points {
            let ssid = ap.ssid.to_string();
            if ssid.is_empty() {
                continue; // Ignore hidden networks for now
            }
            let is_connected = active_ssids.contains(&ssid);
            network_info.push((ssid, is_connected, ap.strength));
        }

        // Sort: connected first, then by strength
        network_info.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2))
        });
        network_info.dedup_by(|a, b| a.0 == b.0);

        for (name, connected, strength) in network_info {
            let signal_icon = if strength > 75 {
                "network-wireless-signal-excellent-symbolic"
            } else if strength > 50 {
                "network-wireless-signal-good-symbolic"
            } else if strength > 25 {
                "network-wireless-signal-ok-symbolic"
            } else {
                "network-wireless-signal-weak-symbolic"
            };

            let status_text = if connected {
                fl!("connected")
            } else {
                String::new()
            };

            let net_row = cosmic::widget::button::custom(
                row![
                    icon::from_name(signal_icon).size(20).symbolic(true),
                    text::body(name.to_string()).width(Length::Fill),
                    text::caption(status_text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .padding(8)
            .on_press(Message::OpenSettings(Some("wifi".to_string())))
            .class(cosmic::theme::Button::Standard);

            network_list = network_list.push(net_row);
        }
    } else {
        network_list = network_list.push(
            padded_control(text::body(fl!("wifi-disabled")).width(Length::Fill))
        );
    }

    let content = column![
        header,
        cosmic::widget::divider::horizontal::default(),
        scrollable(network_list).height(Length::Fill),
    ]
    .spacing(8)
    .padding(8);

    content.into()
}
