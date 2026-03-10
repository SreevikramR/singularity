// SPDX-License-Identifier: GPL-3.0-only
//
// Main dropdown view: the primary AppView::Main layout.
// Section A: Split-button grid (left) + MPRIS media card (right)
// Section B: Volume (with audio submenu) + Brightness + Keyboard Brightness sliders
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
/// All tiles have uniform height and vertically centered content.
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

    // Icon + label, vertically and horizontally centered within the button
    let left_content: Element<'a, Message> = container(
        column![icon_widget, label_widget]
            .spacing(4)
            .align_x(Alignment::Center)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into();

    let left_btn = button::custom(left_content)
        .class(make_style())
        .on_press(on_toggle)
        .width(Length::Fill)
        .height(Length::Fixed(64.0))
        .padding([8, 4]);

    if let Some(nav_msg) = on_navigate {
        let chevron: Element<'a, Message> = container(
            icon::from_name("go-next-symbolic")
                .size(12)
                .symbolic(true)
        )
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .into();

        let right_btn = button::custom(chevron)
            .class(make_style())
            .on_press(nav_msg)
            .height(Length::Fixed(64.0))
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
    // Extract title and artist if available
    let (title, artist) = match player_status {
        Some(status) => {
            let t = status
                .title
                .as_deref()
                .unwrap_or("Unknown Title")
                .to_string();
            let a = status
                .artists
                .as_ref()
                .and_then(|a| a.first())
                .map(|a| a.to_string())
                .unwrap_or_else(|| "Unknown Artist".to_string());
            (t, a)
        }
        None => (fl!("no-media"), String::new()),
    };

    // Placeholder album art icon — centered
    let album_art: Element<'a, Message> = container(
        icon::from_name("applications-multimedia-symbolic")
            .size(48)
            .symbolic(true)
    )
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .padding([4, 0])
    .into();

    let title_text: Element<'a, Message> = text::body(title)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into();

    let mut info_col = column![title_text]
        .spacing(2)
        .width(Length::Fill)
        .align_x(Alignment::Center);

    if !artist.is_empty() {
        let artist_text: Element<'a, Message> = text::caption(artist)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .into();
        info_col = info_col.push(artist_text);
    }

    // Transport controls — always visible, horizontally centered
    let is_playing = player_status
        .as_ref()
        .map(|s| matches!(s.status, PlaybackStatus::Playing))
        .unwrap_or(false);

    let play_pause_icon = if is_playing {
        "media-playback-pause-symbolic"
    } else {
        "media-playback-start-symbolic"
    };
    let play_pause_msg = if is_playing {
        Message::MediaPause
    } else {
        Message::MediaPlay
    };

    let controls: Element<'a, Message> = container(
        row![
            button::icon(icon::from_name("media-skip-backward-symbolic").size(16).symbolic(true))
                .on_press(Message::MediaPrevious)
                .padding(4),
            button::icon(icon::from_name(play_pause_icon).size(20).symbolic(true))
                .on_press(play_pause_msg)
                .padding(4),
            button::icon(icon::from_name("media-skip-forward-symbolic").size(16).symbolic(true))
                .on_press(Message::MediaNext)
                .padding(4),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    )
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .into();

    let card_content: Element<'a, Message> = column![
        album_art,
        info_col,
        controls,
    ]
    .spacing(6)
    .padding(10)
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .into();

    container(card_content)
        .class(theme::Container::Primary)
        .width(Length::Fill)
        .into()
}

// ── Main view assembly ───────────────────────────────────────────────────────

pub fn main_view<'a>(app: &AppModel) -> Element<'a, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;

    // ── Section A: Grid + Media ──────────────────────────────────────────

    // Dynamic power profile icon per state
    let (power_icon, power_label) = match app.power_profile {
        PowerProfile::Balanced => ("power-profile-balanced-symbolic", fl!("balanced")),
        PowerProfile::Performance => ("power-profile-performance-symbolic", fl!("performance")),
        PowerProfile::PowerSaver => ("power-profile-power-saver-symbolic", fl!("power-saver")),
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
            power_icon,
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
            None,
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

    // Volume slider with mute toggle on left and audio submenu chevron on right
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
        button::icon(icon::from_name("go-next-symbolic").size(16).symbolic(true))
            .on_press(Message::Navigate(AppView::AudioDetails))
            .padding(4),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    // Screen brightness with icon
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

    // Keyboard brightness with icon
    let kbd_icon = "keyboard-brightness-symbolic";

    let kbd_brightness_row = row![
        icon::from_name(kbd_icon).size(20).symbolic(true),
        slider(0..=100, app.kbd_brightness, Message::SetKbdBrightness)
            .width(Length::Fill),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    let section_b = column![volume_row, brightness_row, kbd_brightness_row]
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
