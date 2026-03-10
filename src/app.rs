// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::widget::{column, row};
use cosmic::iced::{window, Alignment, Length, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget::{self, button, icon, slider, text};
use cosmic::theme;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<window::Id>,
    /// Configuration data that persists between application runs.
    config: Config,

    // ── Quick-settings tile states ──
    wifi_enabled: bool,
    bluetooth_enabled: bool,
    airplane_mode: bool,
    do_not_disturb: bool,
    dark_mode: bool,
    night_light: bool,

    // ── Sliders ──
    /// Volume level, 0–100.
    volume: u32,
    /// Screen brightness level, 0–100.
    brightness: u32,

    // ── Info display ──
    /// Battery percentage, 0–100.
    battery_percent: f64,
    /// Whether the battery is currently charging.
    battery_charging: bool,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: cosmic::Core::default(),
            popup: None,
            config: Config::default(),
            wifi_enabled: true,
            bluetooth_enabled: true,
            airplane_mode: false,
            do_not_disturb: false,
            dark_mode: false,
            night_light: false,
            volume: 50,
            brightness: 75,
            battery_percent: 85.0,
            battery_charging: true,
        }
    }
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(window::Id),

    // Tile toggles
    ToggleWifi(bool),
    ToggleBluetooth(bool),
    ToggleAirplaneMode(bool),
    ToggleDoNotDisturb(bool),
    ToggleDarkMode(bool),
    ToggleNightLight(bool),

    // Sliders
    SetVolume(u32),
    SetBrightness(u32),

    // System
    UpdateConfig(Config),
    OpenSettings,
    SubscriptionChannel,
}

// ── Tile helper ──────────────────────────────────────────────────────────────

/// Creates a single quick-settings tile widget.
///
/// Each tile is a rounded button with an icon and a label that visually indicates
/// whether the setting is active or inactive.
fn quick_tile<'a>(
    icon_name: &'a str,
    label: String,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let icon_widget: Element<'a, Message> = icon::from_name(icon_name)
        .size(20)
        .symbolic(true)
        .into();

    let label_widget: Element<'a, Message> = text::body(label)
        .width(Length::Shrink)
        .into();

    let content: Element<'a, Message> = column![icon_widget, label_widget]
        .spacing(6)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into();

    let style = if active {
        theme::Button::Suggested
    } else {
        theme::Button::Standard
    };

    button::custom(content)
        .class(style)
        .on_press(on_press)
        .width(Length::Fill)
        .padding(8)
        .into()
}

// ── Application implementation ───────────────────────────────────────────────

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "com.github.sreevikramr.singularity";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        let app = AppModel {
            core,
            config: config.clone(),
            dark_mode: config.dark_mode,
            night_light: config.night_light,
            do_not_disturb: config.do_not_disturb,
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    // ── Panel icon ───────────────────────────────────────────────────────

    /// The applet's button in the panel — a single unified icon.
    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("preferences-system-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    // ── Popup window ─────────────────────────────────────────────────────

    fn view_window(&self, _id: window::Id) -> Element<'_, Self::Message> {
        let spacing = cosmic::theme::active().cosmic().spacing;

        // ── Volume slider ────────────────────────────────────────────────
        let volume_icon = if self.volume == 0 {
            "audio-volume-muted-symbolic"
        } else if self.volume < 33 {
            "audio-volume-low-symbolic"
        } else if self.volume < 66 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-high-symbolic"
        };

        let volume_row = row![
            icon::from_name(volume_icon).size(20).symbolic(true),
            slider(0..=100, self.volume, Message::SetVolume)
                .width(Length::Fill),
        ]
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .padding([0, spacing.space_xs]);

        // ── Brightness slider ────────────────────────────────────────────
        let brightness_icon = if self.brightness < 33 {
            "display-brightness-low-symbolic"
        } else if self.brightness < 66 {
            "display-brightness-medium-symbolic"
        } else {
            "display-brightness-high-symbolic"
        };

        let brightness_row = row![
            icon::from_name(brightness_icon).size(20).symbolic(true),
            slider(0..=100, self.brightness, Message::SetBrightness)
                .width(Length::Fill),
        ]
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .padding([0, spacing.space_xs]);

        // ── Toggle tile grid (3 columns × 2 rows) ───────────────────────
        let tile_row_1 = row![
            quick_tile(
                "network-wireless-symbolic",
                fl!("wifi"),
                self.wifi_enabled,
                Message::ToggleWifi(!self.wifi_enabled),
            ),
            quick_tile(
                "bluetooth-active-symbolic",
                fl!("bluetooth"),
                self.bluetooth_enabled,
                Message::ToggleBluetooth(!self.bluetooth_enabled),
            ),
            quick_tile(
                "airplane-mode-symbolic",
                fl!("airplane-mode"),
                self.airplane_mode,
                Message::ToggleAirplaneMode(!self.airplane_mode),
            ),
        ]
        .spacing(spacing.space_xs)
        .width(Length::Fill);

        let tile_row_2 = row![
            quick_tile(
                "notifications-disabled-symbolic",
                fl!("do-not-disturb"),
                self.do_not_disturb,
                Message::ToggleDoNotDisturb(!self.do_not_disturb),
            ),
            quick_tile(
                "dark-mode-symbolic",
                fl!("dark-mode"),
                self.dark_mode,
                Message::ToggleDarkMode(!self.dark_mode),
            ),
            quick_tile(
                "night-light-symbolic",
                fl!("night-light"),
                self.night_light,
                Message::ToggleNightLight(!self.night_light),
            ),
        ]
        .spacing(spacing.space_xs)
        .width(Length::Fill);

        // ── Battery status ───────────────────────────────────────────────
        let battery_icon = if self.battery_charging {
            "battery-full-charging-symbolic"
        } else if self.battery_percent > 80.0 {
            "battery-full-symbolic"
        } else if self.battery_percent > 50.0 {
            "battery-good-symbolic"
        } else if self.battery_percent > 20.0 {
            "battery-low-symbolic"
        } else {
            "battery-caution-symbolic"
        };

        let battery_label = format!(
            "{:.0}% — {}",
            self.battery_percent,
            if self.battery_charging {
                fl!("charging")
            } else {
                fl!("on-battery")
            }
        );

        let battery_row = row![
            icon::from_name(battery_icon).size(20).symbolic(true),
            text::body(battery_label).width(Length::Fill),
        ]
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .padding([0, spacing.space_xs]);

        // ── Settings button ──────────────────────────────────────────────
        let settings_btn = widget::settings::item(
            fl!("settings"),
            icon::from_name("preferences-system-symbolic")
                .size(20)
                .symbolic(true),
        );

        // ── Assemble the popup ───────────────────────────────────────────
        let content = column![
            // Sliders section
            volume_row,
            brightness_row,
            // Divider
            widget::divider::horizontal::default(),
            // Toggle grid
            tile_row_1,
            tile_row_2,
            // Divider
            widget::divider::horizontal::default(),
            // Battery info
            battery_row,
            // Divider
            widget::divider::horizontal::default(),
            // Settings
            settings_btn,
        ]
        .spacing(spacing.space_xs)
        .padding(spacing.space_xs);

        self.core.applet.popup_container(content).into()
    }

    // ── Subscriptions ────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    // ── Message handling ─────────────────────────────────────────────────

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SubscriptionChannel => {}

            Message::UpdateConfig(config) => {
                self.config = config;
                self.dark_mode = self.config.dark_mode;
                self.night_light = self.config.night_light;
                self.do_not_disturb = self.config.do_not_disturb;
            }

            // ── Tile toggles ─────────────────────────────────────────
            Message::ToggleWifi(v) => self.wifi_enabled = v,
            Message::ToggleBluetooth(v) => self.bluetooth_enabled = v,
            Message::ToggleAirplaneMode(v) => {
                self.airplane_mode = v;
                if v {
                    // Airplane mode disables wireless radios
                    self.wifi_enabled = false;
                    self.bluetooth_enabled = false;
                }
            }
            Message::ToggleDoNotDisturb(v) => self.do_not_disturb = v,
            Message::ToggleDarkMode(v) => self.dark_mode = v,
            Message::ToggleNightLight(v) => self.night_light = v,

            // ── Sliders ──────────────────────────────────────────────
            Message::SetVolume(v) => self.volume = v,
            Message::SetBrightness(v) => self.brightness = v,

            // ── System ───────────────────────────────────────────────
            Message::OpenSettings => {
                // TODO: Launch cosmic-settings via activation token
            }

            // ── Popup lifecycle ──────────────────────────────────────
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = window::Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(380.0)
                        .min_width(340.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
