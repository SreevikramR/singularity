// SPDX-License-Identifier: GPL-3.0-only
//
// Main dropdown view: the primary AppView::Main layout.
// Section A: Split-button grid (left) + MPRIS media card (right)
// Section B: Volume + Brightness sliders
// Section C: Footer with battery info + power buttons

use crate::app::{AppModel, AppView, Message, PowerProfile};
use crate::fl;
use crate::subscriptions::mpris::PlayerStatus;
use cosmic::iced::widget::{column, container, row};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, slider, text};
use cosmic::theme;
use mpris2_zbus::player::PlaybackStatus;

// ── Split-button tile ────────────────────────────────────────────────────────

/// A split button: left side toggles, right side (chevron) navigates.
fn split_tile<'a>(
    icon_name: &'a str,
    label: String,
    active: bool,
    on_toggle: Message,
    on_navigate: Option<Message>,
) -> Element<'a, Message> {
    let make_style = || -> theme::Button {
        if active {
            theme::Button::Suggested
        } else {
            theme::Button::Standard
        }
    };

    let icon_widget: Element<'a, Message> = icon::from_name(icon_name)
        .size(16)
        .symbolic(true)
        .into();

    let label_widget: Element<'a, Message> = text::caption(label)
        .width(Length::Shrink)
        .into();

    // Left part: icon + label (toggles)
    let left_content: Element<'a, Message> = column![icon_widget, label_widget]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into();

    let left_btn = button::custom(left_content)
        .class(make_style())
        .on_press(on_toggle)
        .width(Length::Fill)
        .padding([8, 4]);

    if let Some(nav_msg) = on_navigate {
        let chevron: Element<'a, Message> = icon::from_name("go-next-symbolic")
            .size(12)
            .symbolic(true)
            .into();

        let right_btn = button::custom(chevron)
            .class(make_style())
            .on_press(nav_msg)
            .padding([8, 4]);

        row![left_btn, right_btn]
            .spacing(1)
            .width(Length::Fill)
            .into()
    } else {
        left_btn.into()
    }
}

// ── Media card ───────────────────────────────────────────────────────────────

fn media_card<'a>(player_status: &Option<PlayerStatus>) -> Element<'a, Message> {
    match player_status {
        Some(status) => {
            let title = status
                .title
                .as_deref()
                .unwrap_or("Unknown Title")
                .to_string();
            let artist = status
                .artists
                .as_ref()
                .and_then(|a| a.first())
                .map(|a| a.to_string())
                .unwrap_or_else(|| "Unknown Artist".to_string());

            let title_text: Element<'a, Message> = text::body(title)
                .width(Length::Fill)
                .into();

            let artist_text: Element<'a, Message> = text::caption(artist)
                .width(Length::Fill)
                .into();

            // Transport controls
            let mut controls = row![].spacing(8).align_y(Alignment::Center);

            if status.can_go_previous {
                controls = controls.push(
                    button::icon(icon::from_name("media-skip-backward-symbolic").size(16).symbolic(true))
                        .on_press(Message::MediaPrevious)
                        .padding(4),
                );
            }

            let play_pause_icon = match status.status {
                PlaybackStatus::Playing => "media-playback-pause-symbolic",
                _ => "media-playback-start-symbolic",
            };
            let play_pause_msg = match status.status {
                PlaybackStatus::Playing => Message::MediaPause,
                _ => Message::MediaPlay,
            };
            controls = controls.push(
                button::icon(icon::from_name(play_pause_icon).size(20).symbolic(true))
                    .on_press(play_pause_msg)
                    .padding(4),
            );

            if status.can_go_next {
                controls = controls.push(
                    button::icon(icon::from_name("media-skip-forward-symbolic").size(16).symbolic(true))
                        .on_press(Message::MediaNext)
                        .padding(4),
                );
            }

            let card_content: Element<'a, Message> = column![
                title_text,
                artist_text,
                controls,
            ]
            .spacing(6)
            .padding(10)
            .width(Length::Fill)
            .into();

            container(card_content)
                .class(theme::Container::Primary)
                .width(Length::Fill)
                .into()
        }
        None => {
            let no_media: Element<'a, Message> = column![
                icon::from_name("applications-multimedia-symbolic")
                    .size(24)
                    .symbolic(true),
                text::caption(fl!("no-media")),
            ]
            .spacing(8)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .padding(16)
            .into();

            container(no_media)
                .class(theme::Container::Primary)
                .width(Length::Fill)
                .into()
        }
    }
}

// ── Main view assembly ───────────────────────────────────────────────────────

pub fn main_view<'a>(app: &AppModel) -> Element<'a, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;

    // ── Section A: Grid + Media ──────────────────────────────────────────

    // 2×3 split-button grid
    let power_label = match app.power_profile {
        PowerProfile::Balanced => fl!("balanced"),
        PowerProfile::Performance => fl!("performance"),
        PowerProfile::PowerSaver => fl!("power-saver"),
    };

    let grid_row_1 = row![
        split_tile(
            "network-wireless-symbolic",
            fl!("wifi"),
            app.wifi_enabled,
            Message::ToggleWifi(!app.wifi_enabled),
            Some(Message::Navigate(AppView::WifiDetails)),
        ),
        split_tile(
            "bluetooth-active-symbolic",
            fl!("bluetooth"),
            app.bluetooth_enabled,
            Message::ToggleBluetooth(!app.bluetooth_enabled),
            Some(Message::Navigate(AppView::BluetoothDetails)),
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    let grid_row_2 = row![
        split_tile(
            "power-profile-balanced-symbolic",
            power_label,
            true,
            Message::CyclePowerProfile,
            None,
        ),
        split_tile(
            "network-vpn-symbolic",
            fl!("vpn"),
            app.vpn_active,
            Message::ToggleVpn(!app.vpn_active),
            Some(Message::Navigate(AppView::VpnDetails)),
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    let grid_row_3 = row![
        split_tile(
            "microphone-sensitivity-muted-symbolic",
            fl!("global-mute"),
            app.global_mute,
            Message::ToggleGlobalMute(!app.global_mute),
            Some(Message::Navigate(AppView::AudioDetails)),
        ),
        split_tile(
            "accessories-screenshot-symbolic",
            fl!("screenshot"),
            false,
            Message::TakeScreenshot,
            None,
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    let grid = column![grid_row_1, grid_row_2, grid_row_3]
        .spacing(4)
        .width(Length::FillPortion(1));

    let media = container(media_card(&app.player_status))
        .width(Length::FillPortion(1));

    let section_a = row![grid, media]
        .spacing(spacing.space_xs)
        .width(Length::Fill);

    // ── Section B: Sliders ───────────────────────────────────────────────

    let volume_icon = if app.volume == 0 || app.global_mute {
        "audio-volume-muted-symbolic"
    } else if app.volume < 33 {
        "audio-volume-low-symbolic"
    } else if app.volume < 66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    };

    let volume_row = row![
        button::icon(icon::from_name(volume_icon).size(20).symbolic(true))
            .on_press(Message::ToggleGlobalMute(!app.global_mute))
            .padding(4),
        slider(0..=150, app.volume, Message::SetVolume)
            .width(Length::Fill),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    let brightness_icon = if app.brightness < 33 {
        "display-brightness-low-symbolic"
    } else if app.brightness < 66 {
        "display-brightness-medium-symbolic"
    } else {
        "display-brightness-high-symbolic"
    };

    let brightness_row = row![
        icon::from_name(brightness_icon).size(20).symbolic(true),
        slider(0..=100, app.brightness, Message::SetBrightness)
            .width(Length::Fill),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    let section_b = column![volume_row, brightness_row]
        .spacing(spacing.space_xxs);

    // ── Section C: Footer ────────────────────────────────────────────────

    let battery_icon = if app.battery_charging {
        "battery-full-charging-symbolic"
    } else if app.battery_percent > 80.0 {
        "battery-full-symbolic"
    } else if app.battery_percent > 50.0 {
        "battery-good-symbolic"
    } else if app.battery_percent > 20.0 {
        "battery-low-symbolic"
    } else {
        "battery-caution-symbolic"
    };

    let battery_text = format!(
        "{:.0}% — {}",
        app.battery_percent,
        if app.battery_charging {
            fl!("charging")
        } else {
            fl!("on-battery")
        }
    );

    let battery_info = row![
        icon::from_name(battery_icon).size(16).symbolic(true),
        text::caption(battery_text),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let power_buttons = row![
        button::icon(icon::from_name("system-lock-screen-symbolic").size(16).symbolic(true))
            .on_press(Message::LockScreen)
            .padding(4),
        button::icon(icon::from_name("system-log-out-symbolic").size(16).symbolic(true))
            .on_press(Message::LogOut)
            .padding(4),
        button::icon(icon::from_name("system-shutdown-symbolic").size(16).symbolic(true))
            .on_press(Message::PowerOff)
            .padding(4),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let section_c = row![battery_info, power_buttons]
        .align_y(Alignment::Center)
        .padding([0, spacing.space_xxs]);

    // ── Assemble ─────────────────────────────────────────────────────────

    column![
        section_a,
        cosmic::widget::divider::horizontal::default(),
        section_b,
        cosmic::widget::divider::horizontal::default(),
        section_c,
    ]
    .spacing(spacing.space_xxs)
    .padding(spacing.space_xxs)
    .into()
}
