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
        app.wifi_enabled,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleWifi(!app.wifi_enabled),
    );

    let mut network_list = column![].spacing(4).width(Length::Fill);

    if app.wifi_enabled {
        // Show placeholder networks for now — will be replaced with real NM data
        let demo_networks = [
            ("HomeNetwork", true, 90u8),
            ("Neighbor_5G", false, 65),
            ("CoffeeShop", false, 40),
            ("Airport_WiFi", false, 25),
        ];

        for (name, connected, strength) in demo_networks {
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

            let net_row = padded_control(
                row![
                    icon::from_name(signal_icon).size(20).symbolic(true),
                    text::body(name.to_string()).width(Length::Fill),
                    text::caption(status_text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            );

            network_list = network_list.push(net_row);
        }
    } else {
        network_list = network_list.push(
            padded_control(text::body(String::from("Wi-Fi is disabled")).width(Length::Fill))
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
