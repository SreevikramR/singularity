// SPDX-License-Identifier: GPL-3.0-only
//
// VPN detail view: available VPN connections list.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::views::detail_header::detail_header;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{icon, text};
use cosmic::applet::padded_control;

pub fn vpn_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("vpn-connections"),
        app.vpn_active,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleVpn(!app.vpn_active),
    );

    let mut vpn_list = column![].spacing(4).width(Length::Fill);

    // Placeholder VPN connections
    let demo_vpns = [
        ("Work VPN", true),
        ("Personal WireGuard", false),
        ("Travel VPN", false),
    ];

    for (name, connected) in demo_vpns {
        let status_text = if connected {
            fl!("connected")
        } else {
            String::new()
        };

        let vpn_row = padded_control(
            row![
                icon::from_name("network-vpn-symbolic").size(20).symbolic(true),
                text::body(name.to_string()).width(Length::Fill),
                text::caption(status_text),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );

        vpn_list = vpn_list.push(vpn_row);
    }

    let content = column![
        header,
        cosmic::widget::divider::horizontal::default(),
        scrollable(vpn_list).height(Length::Fill),
    ]
    .spacing(8)
    .padding(8);

    content.into()
}
