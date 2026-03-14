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
use cosmic::iced::widget::Space;
use cosmic::theme;
use mpris2_zbus::player::PlaybackStatus;

// ── Split-button tile ────────────────────────────────────────────────────────

/// A split button: left side toggles, right side (chevron) navigates.
/// All tiles have uniform height and vertically centered content.
fn split_tile<'a>(
    icon_name: &'a str,
    label: String,
    active: bool,
    available: bool,
    on_toggle: Message,
    on_navigate: Option<Message>,
) -> Element<'a, Message> {
    let make_style = move || -> theme::Button {
        if !available {
            theme::Button::Standard
        } else if active {
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

    let mut left_btn = button::custom(left_content)
        .class(make_style())
        .width(Length::Fill)
        .height(Length::Fixed(64.0))
        .padding([8, 4]);

    if available {
        left_btn = left_btn.on_press(on_toggle);
    }

    if let Some(nav_msg) = on_navigate {
        let chevron: Element<'a, Message> = container(
            icon::from_name("go-next-symbolic")
                .size(12)
                .symbolic(true)
        )
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .into();

        let mut right_btn = button::custom(chevron)
            .class(make_style())
            .height(Length::Fixed(64.0))
            .padding([8, 4]);

        if available {
            right_btn = right_btn.on_press(nav_msg);
        }

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

    let can_go_previous = player_status.as_ref().map(|s| s.can_go_previous).unwrap_or(false);
    let can_play = player_status.as_ref().map(|s| s.can_play).unwrap_or(false);
    let can_pause = player_status.as_ref().map(|s| s.can_pause).unwrap_or(false);
    let can_go_next = player_status.as_ref().map(|s| s.can_go_next).unwrap_or(false);

    let mut prev_btn = button::icon(icon::from_name("media-skip-backward-symbolic").size(16).symbolic(true));
    if can_go_previous {
        prev_btn = prev_btn.on_press(Message::MediaPrevious);
    }

    let mut play_pause_btn = button::icon(icon::from_name(play_pause_icon).size(20).symbolic(true));
    if is_playing && can_pause {
        play_pause_btn = play_pause_btn.on_press(Message::MediaPause);
    } else if !is_playing && can_play {
        play_pause_btn = play_pause_btn.on_press(Message::MediaPlay);
    }

    let mut next_btn = button::icon(icon::from_name("media-skip-forward-symbolic").size(16).symbolic(true));
    if can_go_next {
        next_btn = next_btn.on_press(Message::MediaNext);
    }

    let controls: Element<'a, Message> = container(
        row![
            prev_btn,
            play_pause_btn,
            next_btn,
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
            if app.network_available { fl!("wifi") } else { "Unavailable".to_string() },
            app.nm_state.wifi_enabled,
            app.network_available,
            Message::ToggleWifi(!app.nm_state.wifi_enabled),
            if app.network_available { Some(Message::Navigate(AppView::WifiDetails)) } else { None },
        ),
        split_tile(
            "bluetooth-active-symbolic",
            if app.bluetooth_available { fl!("bluetooth") } else { "Unavailable".to_string() },
            app.bluer_state.bluetooth_enabled,
            app.bluetooth_available,
            Message::ToggleBluetooth(!app.bluer_state.bluetooth_enabled),
            if app.bluetooth_available { Some(Message::Navigate(AppView::BluetoothDetails)) } else { None },
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    let grid_row_2 = row![
        split_tile(
            power_icon,
            power_label,
            true,
            true,
            Message::CyclePowerProfile,
            None,
        ),
        split_tile(
            "network-vpn-symbolic",
            if app.network_available { fl!("vpn") } else { "Unavailable".to_string() },
            app.vpn_active,
            app.network_available,
            Message::ToggleVpn(!app.vpn_active),
            if app.network_available { Some(Message::Navigate(AppView::VpnDetails)) } else { None },
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    let grid_row_3 = row![
        split_tile(
            "microphone-sensitivity-muted-symbolic",
            if app.sound_available { fl!("global-mute") } else { "Unavailable".to_string() },
            app.global_mute,
            app.sound_available,
            Message::ToggleGlobalMute(!app.global_mute),
            None,
        ),
        split_tile(
            "accessories-screenshot-symbolic",
            fl!("screenshot"),
            false,
            true,
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

    let mut volume_row = row![
        button::icon(icon::from_name(volume_icon).size(20).symbolic(true))
            .on_press(Message::ToggleGlobalMute(!app.global_mute))
            .padding(4)
            .width(Length::Fixed(28.0)),
        slider(0..=app.config.max_volume, app.volume, Message::SetVolume)
            .width(Length::Fill),
    ];
    if app.sound_available {
        volume_row = volume_row.push(
            button::icon(icon::from_name("go-next-symbolic").size(16).symbolic(true))
                .on_press(Message::Navigate(AppView::AudioDetails))
                .padding(4)
                .width(Length::Fixed(24.0))
        );
    } else {
        volume_row = volume_row.push(Space::new().width(Length::Fixed(24.0)));
    }
    let volume_row = volume_row
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
        container(icon::from_name(brightness_icon).size(20).symbolic(true))
            .padding(4)
            .width(Length::Fixed(28.0)),
        slider(0..=100, app.brightness, Message::SetBrightness)
            .width(Length::Fill),
        Space::new().width(Length::Fixed(24.0)),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    // Keyboard brightness with icon
    let kbd_icon = "keyboard-brightness-symbolic";

    let kbd_brightness_row = row![
        container(icon::from_name(kbd_icon).size(20).symbolic(true))
            .padding(4)
            .width(Length::Fixed(28.0)),
        slider(0..=100, app.kbd_brightness, Message::SetKbdBrightness)
            .width(Length::Fill),
        Space::new().width(Length::Fixed(24.0)),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    let mut section_b = column![volume_row].spacing(spacing.space_xxs);

    // Microphone slider
    let mic_icon = if app.sound.source_volume == 0 || app.sound.source_mute {
        "microphone-sensitivity-muted-symbolic"
    } else if app.sound.source_volume < 33 {
        "microphone-sensitivity-low-symbolic"
    } else if app.sound.source_volume < 66 {
        "microphone-sensitivity-medium-symbolic"
    } else {
        "microphone-sensitivity-high-symbolic"
    };

    let mic_row = row![
        button::icon(icon::from_name(mic_icon).size(20).symbolic(true))
            .on_press(Message::ToggleSourceMute)
            .padding(4)
            .width(Length::Fixed(28.0)),
        slider(0..=app.max_source_volume, app.sound.source_volume, Message::SetSourceVolume)
            .width(Length::Fill),
        Space::new().width(Length::Fixed(24.0)),
    ]
    .spacing(spacing.space_s)
    .align_y(Alignment::Center)
    .padding([0, spacing.space_xxs]);

    section_b = section_b.push(mic_row);

    if app.max_brightness > 0 {
        section_b = section_b.push(brightness_row);
    }
    if app.max_kbd_brightness > 0 {
        section_b = section_b.push(kbd_brightness_row);
    }

    // ── Section C: Footer ────────────────────────────────────────────────

    let battery_text = if app.battery_charging && app.battery_percent >= 99.0 {
        fl!("fully-charged")
    } else if app.battery_charging {
        if let Some(secs) = app.time_to_full {
            let hours = secs / 3600;
            let minutes = (secs % 3600) / 60;
            if hours == 0 && minutes == 0 {
                format!("{:.0}% — {}", app.battery_percent, fl!("charging"))
            } else if hours == 0 {
                format!("{:.0}% — {}m until full", app.battery_percent, minutes)
            } else if minutes == 0 {
                format!("{:.0}% — {}h until full", app.battery_percent, hours)
            } else {
                format!("{:.0}% — {}h {}m until full", app.battery_percent, hours, minutes)
            }
        } else {
            format!("{:.0}% — {}", app.battery_percent, fl!("charging"))
        }
    } else if let Some(secs) = app.time_to_empty {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        if hours == 0 && minutes == 0 {
            format!("{:.0}% — Less than a minute remaining", app.battery_percent)
        } else if hours == 0 {
            format!("{:.0}% — {}m remaining", app.battery_percent, minutes)
        } else if minutes == 0 {
            format!("{:.0}% — {}h remaining", app.battery_percent, hours)
        } else {
            format!("{:.0}% — {}h {}m remaining", app.battery_percent, hours, minutes)
        }
    } else {
        format!("{:.0}% — {}", app.battery_percent, fl!("on-battery"))
    };

    let battery_info: Element<'_, Message> = if let Some(battery_icon) = app.battery_icon_name() {
        row![
            icon::from_name(battery_icon).size(16).symbolic(true),
            text::caption(battery_text),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
    } else {
        Space::new().width(Length::Fill).into()
    };

    let power_buttons = row![
        button::icon(icon::from_name("preferences-system-symbolic").size(16).symbolic(true))
            .on_press(Message::OpenSettings(None)),
        Space::new().width(Length::Fill),
        button::icon(icon::from_name("system-lock-screen-symbolic").size(16).symbolic(true))
            .on_press(Message::LockScreen),
        button::icon(icon::from_name("system-log-out-symbolic").size(16).symbolic(true))
            .on_press(Message::LogOut),
        button::icon(icon::from_name("system-suspend-symbolic").size(16).symbolic(true))
            .on_press(Message::Suspend),
        button::icon(icon::from_name("system-shutdown-symbolic").size(16).symbolic(true))
            .on_press(Message::PowerOff),
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
