// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::subscriptions::mpris::{self, MprisUpdate, PlayerStatus};
use crate::views;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;

use std::time::Duration;

// Re-export subscription event types
use cosmic_settings_daemon_subscription as brightness_sub;
use cosmic_settings_upower_subscription::device::{self as upower_device, DeviceDbusEvent};
use cosmic_settings_upower_subscription::kbdbacklight::{
    self as upower_kbd, KeyboardBacklightRequest, KeyboardBacklightUpdate,
};

// ── View routing ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AppView {
    #[default]
    Main,
    WifiDetails,
    BluetoothDetails,
    VpnDetails,
    AudioDetails,
}

// ── Power profiles ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PowerProfile {
    #[default]
    Balanced,
    Performance,
    PowerSaver,
}

impl PowerProfile {
    pub fn next(&self) -> Self {
        match self {
            Self::Balanced => Self::Performance,
            Self::Performance => Self::PowerSaver,
            Self::PowerSaver => Self::Balanced,
        }
    }
}

// ── Application state ────────────────────────────────────────────────────────

pub struct AppModel {
    /// Application state managed by the COSMIC runtime.
    pub core: cosmic::Core,
    /// Popup window id.
    popup: Option<window::Id>,
    /// Persisted configuration.
    config: Config,
    /// Current view being rendered in the popup.
    pub current_view: AppView,

    // ── Quick-settings tile states ──
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub vpn_active: bool,
    pub global_mute: bool,
    pub power_profile: PowerProfile,

    // ── Sliders ──
    pub volume: u32,
    pub brightness: u32,
    pub max_brightness: i32,
    pub kbd_brightness: u32,
    pub max_kbd_brightness: i32,

    // ── Senders for settings-daemon control ──
    pub brightness_sender: Option<tokio::sync::mpsc::UnboundedSender<brightness_sub::Request>>,
    pub kbd_brightness_sender:
        Option<tokio::sync::mpsc::UnboundedSender<KeyboardBacklightRequest>>,

    // ── Battery ──
    pub battery_percent: f64,
    pub battery_charging: bool,
    pub has_battery: bool,

    // ── MPRIS ──
    pub player_status: Option<PlayerStatus>,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: cosmic::Core::default(),
            popup: None,
            config: Config::default(),
            current_view: AppView::Main,
            wifi_enabled: true,
            bluetooth_enabled: true,
            vpn_active: false,
            global_mute: false,
            power_profile: PowerProfile::Balanced,
            volume: 50,
            brightness: 75,
            max_brightness: 100,
            kbd_brightness: 50,
            max_kbd_brightness: 100,
            brightness_sender: None,
            kbd_brightness_sender: None,
            battery_percent: 0.0,
            battery_charging: false,
            has_battery: true,
            player_status: None,
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // Popup lifecycle
    TogglePopup,
    PopupClosed(window::Id),

    // Navigation
    Navigate(AppView),

    // Tile toggles
    ToggleWifi(bool),
    ToggleBluetooth(bool),
    ToggleVpn(bool),
    ToggleGlobalMute(bool),
    CyclePowerProfile,

    // Sliders
    SetVolume(u32),
    SetBrightness(u32),
    SetKbdBrightness(u32),

    // Media controls
    MediaPlay,
    MediaPause,
    MediaNext,
    MediaPrevious,
    MprisEvent(MprisUpdate),

    // Screenshot
    TakeScreenshot,
    ExecuteScreenshot,

    // Power actions
    LockScreen,
    LogOut,
    PowerOff,

    // System
    UpdateConfig(Config),

    // ── D-Bus subscription events ──
    UPowerDevice(DeviceDbusEvent),
    KbdBacklight(KeyboardBacklightUpdate),
    Brightness(brightness_sub::Event),
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
            config,
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    // ── Panel button: pill-shaped with 3 dynamic icons ───────────────────

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("preferences-system-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    // ── Popup: route to active view ──────────────────────────────────────

    fn view_window(&self, _id: window::Id) -> Element<'_, Self::Message> {
        let content = match &self.current_view {
            AppView::Main => views::main_view::main_view(self),
            AppView::WifiDetails => views::wifi_details::wifi_details_view(self),
            AppView::BluetoothDetails => views::bluetooth_details::bluetooth_details_view(self),
            AppView::VpnDetails => views::vpn_details::vpn_details_view(self),
            AppView::AudioDetails => views::audio_details::audio_details_view(self),
        };
        self.core.applet.popup_container(content).into()
    }

    // ── Subscriptions ────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            // Config watcher
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
            // MPRIS media player
            mpris::mpris_subscription(0u8).map(Message::MprisEvent),
            // UPower battery
            upower_device::device_subscription(1u8).map(Message::UPowerDevice),
            // UPower keyboard backlight
            upower_kbd::kbd_backlight_subscription(2u8).map(Message::KbdBacklight),
            // Settings daemon (display brightness)
            brightness_sub::subscription().map(Message::Brightness),
        ])
    }

    // ── Message handling ─────────────────────────────────────────────────

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            // ── Navigation ───────────────────────────────────────────
            Message::Navigate(view) => {
                self.current_view = view;
            }

            // ── Config ───────────────────────────────────────────────
            Message::UpdateConfig(config) => {
                self.config = config;
            }

            // ── Tile toggles ─────────────────────────────────────────
            Message::ToggleWifi(v) => self.wifi_enabled = v,
            Message::ToggleBluetooth(v) => self.bluetooth_enabled = v,
            Message::ToggleVpn(v) => self.vpn_active = v,
            Message::ToggleGlobalMute(v) => self.global_mute = v,
            Message::CyclePowerProfile => {
                self.power_profile = self.power_profile.next();
            }

            // ── Sliders ──────────────────────────────────────────────
            Message::SetVolume(v) => self.volume = v,
            Message::SetBrightness(v) => {
                self.brightness = v;
                // Send brightness change to settings daemon
                if let Some(ref sender) = self.brightness_sender {
                    let raw = (v as i32 * self.max_brightness) / 100;
                    let _ = sender.send(brightness_sub::Request::SetDisplayBrightness(raw));
                }
            }
            Message::SetKbdBrightness(v) => {
                self.kbd_brightness = v;
                // Send keyboard brightness change to UPower
                if let Some(ref sender) = self.kbd_brightness_sender {
                    let raw = (v as i32 * self.max_kbd_brightness) / 100;
                    let _ = sender.send(KeyboardBacklightRequest::Set(raw));
                }
            }

            // ── D-Bus: UPower battery ────────────────────────────────
            Message::UPowerDevice(event) => match event {
                DeviceDbusEvent::NoBattery => {
                    self.has_battery = false;
                }
                DeviceDbusEvent::Update {
                    on_battery,
                    percent,
                    time_to_empty: _,
                } => {
                    self.has_battery = true;
                    self.battery_charging = !on_battery;
                    self.battery_percent = percent;
                }
            },

            // ── D-Bus: Keyboard backlight ────────────────────────────
            Message::KbdBacklight(update) => match update {
                KeyboardBacklightUpdate::Sender(sender) => {
                    self.kbd_brightness_sender = Some(sender);
                }
                KeyboardBacklightUpdate::Brightness(val) => {
                    if self.max_kbd_brightness > 0 {
                        self.kbd_brightness =
                            ((val as f64 / self.max_kbd_brightness as f64) * 100.0) as u32;
                    }
                }
                KeyboardBacklightUpdate::MaxBrightness(val) => {
                    self.max_kbd_brightness = val;
                }
            },

            // ── D-Bus: Display brightness ────────────────────────────
            Message::Brightness(event) => match event {
                brightness_sub::Event::Sender(sender) => {
                    self.brightness_sender = Some(sender);
                }
                brightness_sub::Event::DisplayBrightness(val) => {
                    if self.max_brightness > 0 {
                        self.brightness =
                            ((val as f64 / self.max_brightness as f64) * 100.0) as u32;
                    }
                }
                brightness_sub::Event::MaxDisplayBrightness(val) => {
                    self.max_brightness = val;
                }
            },

            // ── MPRIS ────────────────────────────────────────────────
            Message::MprisEvent(update) => match update {
                MprisUpdate::Player(status) => {
                    self.player_status = Some(status);
                }
                MprisUpdate::Finished => {
                    self.player_status = None;
                }
                MprisUpdate::Setup => {}
            },
            Message::MediaPlay => {
                if let Some(ref status) = self.player_status {
                    let player = status.player.clone();
                    return Task::perform(
                        async move {
                            let _ = player.play().await;
                        },
                        |_| cosmic::Action::App(Message::MprisEvent(MprisUpdate::Setup)),
                    );
                }
            }
            Message::MediaPause => {
                if let Some(ref status) = self.player_status {
                    let player = status.player.clone();
                    return Task::perform(
                        async move {
                            let _ = player.pause().await;
                        },
                        |_| cosmic::Action::App(Message::MprisEvent(MprisUpdate::Setup)),
                    );
                }
            }
            Message::MediaNext => {
                if let Some(ref status) = self.player_status {
                    let player = status.player.clone();
                    return Task::perform(
                        async move {
                            let _ = player.next().await;
                        },
                        |_| cosmic::Action::App(Message::MprisEvent(MprisUpdate::Setup)),
                    );
                }
            }
            Message::MediaPrevious => {
                if let Some(ref status) = self.player_status {
                    let player = status.player.clone();
                    return Task::perform(
                        async move {
                            let _ = player.previous().await;
                        },
                        |_| cosmic::Action::App(Message::MprisEvent(MprisUpdate::Setup)),
                    );
                }
            }

            // ── Screenshot ───────────────────────────────────────────
            Message::TakeScreenshot => {
                // Close the popup first, then schedule screenshot after delay
                let close_task = if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    Task::none()
                };

                let screenshot_task = Task::perform(
                    async {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    },
                    |_| cosmic::Action::App(Message::ExecuteScreenshot),
                );

                return Task::batch([close_task, screenshot_task]);
            }
            Message::ExecuteScreenshot => {
                let cmd = if self.config.screenshot_command.is_empty() {
                    "cosmic-screenshot".to_string()
                } else {
                    self.config.screenshot_command.clone()
                };

                return Task::perform(
                    async move {
                        let _ = tokio::process::Command::new("sh")
                            .arg("-c")
                            .arg(&cmd)
                            .spawn();
                    },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }

            // ── Power actions ────────────────────────────────────────
            Message::LockScreen => {
                return Task::perform(
                    async { let _ = crate::subscriptions::power::lock_screen().await; },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }
            Message::LogOut => {
                return Task::perform(
                    async { let _ = crate::subscriptions::power::log_out().await; },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }
            Message::PowerOff => {
                return Task::perform(
                    async { let _ = crate::subscriptions::power::suspend().await; },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }

            // ── Popup lifecycle ──────────────────────────────────────
            Message::TogglePopup => {
                // Reset to main view when toggling popup
                self.current_view = AppView::Main;

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
                        .max_width(460.0)
                        .min_width(400.0)
                        .min_height(300.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.current_view = AppView::Main;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
