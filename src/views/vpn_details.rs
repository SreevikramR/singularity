// SPDX-License-Identifier: GPL-3.0-only
//
// VPN detail view: available VPN connections list.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::network::ConnectionSettings;
use crate::views::detail_header::detail_header;
use cosmic::applet::padded_control;
use cosmic::iced::widget::{column, row, scrollable};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, text, text_input};
use cosmic_settings_network_manager_subscription::ActiveConnectionInfo;

pub fn vpn_details_view(app: &AppModel) -> Element<'_, Message> {
    let header = detail_header(
        &fl!("vpn-connections"),
        app.vpn_active,
        Message::Navigate(crate::app::AppView::Main),
        Message::ToggleVpn(!app.vpn_active),
    );

    let mut vpn_list = column![].spacing(8).width(Length::Fill);

    if let Some(requested_vpn) = app.requested_vpn.as_ref() {
        let title = requested_vpn
            .description
            .as_deref()
            .unwrap_or(requested_vpn.uuid.as_ref());

        let id_row = row![
            icon::from_name("network-vpn-symbolic")
                .size(24)
                .symbolic(true),
            text::body(title),
        ]
        .align_y(Alignment::Center)
        .spacing(12);

        let pass_col = column![
            text_input::secure_input(
                "",
                requested_vpn.password.unsecure(),
                Some(Message::ToggleVpnPasswordVisibility),
                requested_vpn.password_hidden,
            )
            .on_input(|s| Message::VPNPasswordUpdate(s.into()))
            .on_paste(|s| Message::VPNPasswordUpdate(s.into()))
            .on_submit(|_| Message::ConnectVPNWithPassword),
            row![
                button::standard(fl!("cancel")).on_press(Message::CancelVPNConnection),
                button::suggested(fl!("connect")).on_press(Message::ConnectVPNWithPassword)
            ]
            .spacing(24)
        ]
        .spacing(12);

        vpn_list = vpn_list
            .push(padded_control(id_row))
            .push(padded_control(pass_col));
    } else {
        if app.known_vpns.is_empty() {
            vpn_list = vpn_list.push(
                cosmic::widget::container(padded_control(text::body("No VPN connections found")))
                    .width(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .padding(16),
            );
        }

        for (uuid, connection) in &app.known_vpns {
            let id = match connection {
                ConnectionSettings::Vpn(connection) => connection.id.as_str(),
                ConnectionSettings::Wireguard { id } => id.as_str(),
            };

            let is_active = app.nm_state.active_conns.iter().any(|conn| {
                matches!(conn, ActiveConnectionInfo::Vpn { name, .. } if name == id)
            });

            let mut right_side = vec![];

            if is_active {
                right_side.push(text::body(fl!("connected")).into());
                right_side.push(
                    icon::from_name("object-select-symbolic")
                        .size(16)
                        .symbolic(true)
                        .into(),
                );
            }

            let btn_content = row![
                icon::from_name("network-vpn-symbolic")
                    .size(24)
                    .symbolic(true),
                text::body(id).width(Length::Fill),
                cosmic::iced::widget::Row::with_children(right_side)
                    .spacing(8)
                    .align_y(Alignment::Center)
            ]
            .align_y(Alignment::Center)
            .spacing(12);

            let mut btn = cosmic::widget::button::custom(btn_content)
                .width(Length::Fill)
                .padding(12)
                .class(cosmic::theme::Button::Standard);

            btn = if is_active {
                btn.on_press(Message::DeactivateVpn(uuid.clone()))
            } else {
                btn.on_press(Message::ActivateVpn(uuid.clone()))
            };

            vpn_list = vpn_list.push(btn);
        }
    }

    let content = column![
        header,
        cosmic::widget::divider::horizontal::default(),
        scrollable(vpn_list.padding(8)).height(Length::Fill),
    ]
    .spacing(4);

    content.into()
}
