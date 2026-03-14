// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::subscriptions::mpris::{self, MprisUpdate, PlayerStatus};
use crate::views;
use crate::bluetooth::{self, BluerState, BluerRequest, BluerEvent, BluerAgentEvent, BluerDevice};
use crate::network::*;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window, Limits, Subscription};

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub enum IsOpen {
    #[default]
    None,
    Output,
    Input,
}
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;


// Re-export subscription event types
use cosmic_settings_daemon_subscription as brightness_sub;
use cosmic_settings_upower_subscription::device::{self as upower_device, DeviceDbusEvent};
use cosmic_settings_upower_subscription::kbdbacklight::{
    self as upower_kbd, KeyboardBacklightRequest, KeyboardBacklightUpdate,
};
use cosmic_settings_network_manager_subscription::{
    self as nm_sub, Event as NmEvent, NetworkManagerState, Request as NmRequest,
    nm_secret_agent, UUID, hw_address::HwAddress, available_wifi::AccessPoint,
    current_networks::ActiveConnectionInfo,
};
use indexmap::IndexMap;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use std::sync::Arc;
use zbus::Connection;
use secure_string::SecureString;

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
    pub config: Config,
    /// Current view being rendered in the popup.
    pub current_view: AppView,

    // ── Quick-settings tile states ──
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
    pub time_to_empty: Option<i64>,
    pub time_to_full: Option<i64>,

    // ── NetworkManager ──
    pub nm_state: NetworkManagerState,
    pub nm_sender: Option<futures::channel::mpsc::UnboundedSender<NmRequest>>,
    pub show_visible_networks: bool,
    pub new_connection: Option<NewConnectionState>,
    pub conn: Option<Connection>,
    pub secret_tx: Option<tokio::sync::mpsc::Sender<nm_secret_agent::Request>>,
    pub known_vpns: IndexMap<UUID, ConnectionSettings>,
    pub ssid_to_uuid: BTreeMap<Box<str>, Box<str>>,
    pub failed_known_ssids: FxHashSet<Arc<str>>,
    pub requested_vpn: Option<RequestedVpn>,
    pub nm_task: Option<tokio::sync::oneshot::Sender<()>>,
    pub nm_devices: Vec<Arc<cosmic_settings_network_manager_subscription::devices::DeviceInfo>>,

    // ── MPRIS ──
    pub player_status: Option<PlayerStatus>,

    // ── Sound ──
    pub sound: cosmic_settings_sound_subscription::Model,
    pub is_open: IsOpen,
    pub max_sink_volume: u32,
    pub max_source_volume: u32,

    // ── Bluetooth (Bluer) ──
    pub bluer_state: BluerState,
    pub bluer_sender: Option<tokio::sync::mpsc::Sender<BluerRequest>>,
    pub show_visible_devices: bool,
    pub request_confirmation: Option<(BluerDevice, String, tokio::sync::mpsc::Sender<bool>)>,

    // ── Service Availability ──
    pub bluetooth_available: bool,
    pub network_available: bool,
    pub sound_available: bool,
    
    // ── Screenshot ──
    pub pending_screenshot: bool,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: cosmic::Core::default(),
            popup: None,
            config: Config::default(),
            current_view: AppView::Main,
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
            time_to_empty: None,
            time_to_full: None,
            nm_state: NetworkManagerState::default(),
            nm_sender: None,
            show_visible_networks: false,
            new_connection: None,
            conn: None,
            secret_tx: None,
            known_vpns: IndexMap::new(),
            ssid_to_uuid: BTreeMap::new(),
            failed_known_ssids: FxHashSet::default(),
            requested_vpn: None,
            nm_task: None,
            nm_devices: Vec::new(),
            player_status: None,
            sound: Default::default(),
            is_open: IsOpen::None,
            max_sink_volume: 150, // Arbitrary default or cosmic config
            max_source_volume: 150,
            bluer_state: Default::default(),
            bluer_sender: None,
            show_visible_devices: false,
            request_confirmation: None,
            bluetooth_available: false,
            network_available: false,
            sound_available: false,
            pending_screenshot: false,
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
    SetSourceVolume(u32),
    SetBrightness(u32),
    SetKbdBrightness(u32),
    SetDefaultSink(usize),
    SetDefaultSource(usize),

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
    Suspend,
    OpenSettings(Option<String>),

    // Sound specific toggles
    OutputToggle,
    InputToggle,
    ToggleSourceMute,

    // System
    UpdateConfig(Config),

    // ── D-Bus subscription events ──
    UPowerDevice(DeviceDbusEvent),
    KbdBacklight(KeyboardBacklightUpdate),
    Brightness(brightness_sub::Event),
    NetworkManager(NmEvent),
    Sound(cosmic_settings_sound_subscription::Message),
    
    // ── Network Manager Detailed Requests ──
    ToggleVisibleNetworks,
    SelectWirelessAccessPoint(AccessPoint),
    CancelNewConnection,
    Connect(nm_sub::SSID, HwAddress),
    ConnectWithPassword,
    Disconnect(nm_sub::SSID, HwAddress),
    PasswordUpdate(SecureString),
    IdentityUpdate(String),
    TogglePasswordVisibility,
    SecretAgent(nm_secret_agent::Event),
    NetworkManagerConnect(Connection),
    ConnectionSettings(BTreeMap<Box<str>, Box<str>>),
    KnownConnections(IndexMap<UUID, ConnectionSettings>),
    ResetFailedKnownSsid(String, HwAddress),
    ActivateVpn(Arc<str>),
    DeactivateVpn(Arc<str>),
    ToggleVpnPasswordVisibility,
    ConnectVPNWithPassword,
    VPNPasswordUpdate(SecureString),
    CancelVPNConnection,
    UpdateState(NetworkManagerState),
    UpdateDevices(Vec<cosmic_settings_network_manager_subscription::devices::DeviceInfo>),
    Refresh,
    Error(String),
    
    // ── Bluer Bluetooth ──
    BluetoothEvent(BluerEvent),
    BluetoothRequest(BluerRequest),
    ConfirmPairing,
    CancelPairing,
    ToggleVisibleDevices(bool),
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
        tracing::info!("Initializing Singularity applet");
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

        // Bluetooth discovery doesn't need an init task like the subscription crate did
        (app, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    // ── Panel button: pill-shaped with 3 dynamic icons ───────────────────

    fn view(&self) -> Element<'_, Self::Message> {
        let volume_icon = if self.volume == 0 || self.global_mute {
            "audio-volume-muted-symbolic"
        } else if self.volume < 33 {
            "audio-volume-low-symbolic"
        } else if self.volume < 66 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-high-symbolic"
        };

        let mut wifi_icon = "network-wireless-offline-symbolic";
        if self.nm_state.wifi_enabled {
            wifi_icon = "network-wireless-disconnected-symbolic";
            
            let mut active_ssid = None;
            for conn in &self.nm_state.active_conns {
                if let nm_sub::ActiveConnectionInfo::WiFi { name, .. } = conn {
                    active_ssid = Some(name.clone());
                    break;
                }
            }

            if let Some(ssid) = active_ssid {
                wifi_icon = "network-wireless-symbolic";
                for ap in &self.nm_state.wireless_access_points {
                    if ap.ssid.as_ref() == ssid.as_str() {
                        if ap.strength > 75 {
                            wifi_icon = "network-wireless-signal-excellent-symbolic";
                        } else if ap.strength > 50 {
                            wifi_icon = "network-wireless-signal-good-symbolic";
                        } else if ap.strength > 25 {
                            wifi_icon = "network-wireless-signal-ok-symbolic";
                        } else {
                            wifi_icon = "network-wireless-signal-weak-symbolic";
                        }
                        break;
                    }
                }
            }
        }

        let mut battery_icon = None;
        if self.has_battery {
            battery_icon = Some("battery-symbolic");
        }

        let mut icons_row = cosmic::iced::widget::row![
            cosmic::widget::icon::from_name(volume_icon).size(16).symbolic(true),
            cosmic::widget::icon::from_name(wifi_icon).size(16).symbolic(true),
        ]
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center);

        if let Some(bat_icon) = battery_icon {
            icons_row = icons_row.push(cosmic::widget::icon::from_name(bat_icon).size(16).symbolic(true));
        }

        let button = cosmic::widget::button::custom(icons_row)
            .class(cosmic::theme::Button::AppletIcon)
            .on_press_down(Message::TogglePopup);

        cosmic::widget::autosize::autosize(button, cosmic::widget::Id::unique()).into()
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
        let mut subs = vec![
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
            // Network Manager
            nm_sub::subscription().map(Message::NetworkManager),
            // Sound/Audio
            Subscription::run_with("singularity-sound", get_sound_watch).map(Message::Sound),
            // Bluetooth
            bluetooth::bluetooth_subscription(3u8).map(Message::BluetoothEvent),
        ];
        if self.popup.is_some() {
            subs.push(
                cosmic::iced::time::every(std::time::Duration::from_secs(4))
                    .map(|_| Message::Refresh)
            );
        }
        Subscription::batch(subs)
    }

    // ── Message handling ─────────────────────────────────────────────────

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        tracing::debug!("Received message: {:?}", message);
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
            Message::ToggleWifi(v) => {
                if let Some(ref sender) = self.nm_sender {
                    let _ = sender.unbounded_send(NmRequest::SetWiFi(v));
                }
            }
            Message::ToggleBluetooth(v) => {
                return Task::perform(
                    async move { v },
                    |enabled| cosmic::Action::App(Message::BluetoothRequest(BluerRequest::SetBluetoothEnabled(enabled))),
                );
            }
            Message::ToggleVpn(v) => self.vpn_active = v,
            Message::ToggleGlobalMute(_v) => {
                let _ = std::process::Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).spawn();
                self.sound.toggle_sink_mute();
                self.global_mute = self.sound.sink_mute;
            }
            Message::ToggleSourceMute => {
                let _ = std::process::Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"]).spawn();
                self.sound.toggle_source_mute();
            }
            Message::CyclePowerProfile => {
                self.power_profile = self.power_profile.next();
            }

            // ── Sliders ──────────────────────────────────────────────
            Message::InputToggle => {
                self.is_open = if self.is_open == IsOpen::Input {
                    IsOpen::None
                } else {
                    IsOpen::Input
                };
            }
            Message::OutputToggle => {
                self.is_open = if self.is_open == IsOpen::Output {
                    IsOpen::None
                } else {
                    IsOpen::Output
                };
            }
            Message::SetVolume(v) => {
                self.volume = v;
                return self.sound.set_sink_volume(v)
                    .map(|msg| cosmic::Action::App(Message::Sound(msg)));
            }
            Message::SetSourceVolume(v) => {
                return self.sound.set_source_volume(v)
                    .map(|msg| cosmic::Action::App(Message::Sound(msg)));
            }
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
            Message::SetDefaultSink(pos) => {
                return self.sound.set_default_sink(pos).map(|m| cosmic::Action::App(Message::Sound(m)));
            }
            Message::SetDefaultSource(pos) => {
                return self.sound.set_default_source(pos).map(|m| cosmic::Action::App(Message::Sound(m)));
            }

            // ── D-Bus: UPower battery ────────────────────────────────
            Message::UPowerDevice(event) => match event {
                DeviceDbusEvent::NoBattery => {
                    self.has_battery = false;
                }
                DeviceDbusEvent::Update {
                    on_battery,
                    percent,
                    time_to_empty,
                    time_to_full,
                } => {
                    self.has_battery = true;
                    self.battery_charging = !on_battery;
                    self.battery_percent = percent;
                    if on_battery {
                        self.time_to_empty = Some(time_to_empty);
                        self.time_to_full = None;
                    } else {
                        self.time_to_empty = None;
                        self.time_to_full = Some(time_to_full);
                    }
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

            // ── D-Bus: Network Manager ───────────────────────────────
            Message::NetworkManager(event) => {
                match event {
                    NmEvent::Init { sender, state, conn } => {
                        self.network_available = true;
                        self.nm_sender = Some(sender);
                        self.nm_state = state;
                        self.conn = Some(conn);
                        self.vpn_active = self.nm_state.active_conns.iter().any(|c| matches!(c, cosmic_settings_network_manager_subscription::ActiveConnectionInfo::Vpn { .. }));
                        if let Some(ref sender) = self.nm_sender {
                            let _ = sender.unbounded_send(NmRequest::Reload);
                        }
                        return self.update(Message::Refresh);
                    }
                    NmEvent::RequestResponse { state, success, req } => {
                        if !success {
                            tracing::error!("NetworkManager request error on {:?}", req);
                        }
                        
                        match &req {
                            nm_sub::Request::SelectAccessPoint(ssid, _, _, _) => {
                                let conn_match = self.new_connection.as_ref().is_some_and(|c| {
                                    match c {
                                        NewConnectionState::EnterPassword { access_point, .. } => access_point.ssid.as_ref() == ssid.as_ref(),
                                        NewConnectionState::Waiting(access_point) => access_point.ssid.as_ref() == ssid.as_ref(),
                                        NewConnectionState::Failure(access_point) => access_point.ssid.as_ref() == ssid.as_ref(),
                                    }
                                });

                                if conn_match && success {
                                    self.new_connection = None;
                                    self.show_visible_networks = false;
                                    self.failed_known_ssids.remove(ssid.as_ref());
                                } else if !matches!(&self.new_connection, Some(NewConnectionState::EnterPassword { .. })) && !success {
                                    self.failed_known_ssids.insert(ssid.as_ref().into());
                                }
                            }
                            nm_sub::Request::Authenticate { ssid, .. } => {
                                if let Some(NewConnectionState::Waiting(access_point)) = self.new_connection.as_ref() {
                                    if !success && ssid.as_str() == access_point.ssid.as_ref() {
                                        self.new_connection = Some(NewConnectionState::Failure(access_point.clone()));
                                    } else {
                                        self.show_visible_networks = false;
                                    }
                                } else if let Some(NewConnectionState::EnterPassword { access_point, .. }) = self.new_connection.as_ref() {
                                    if success && ssid.as_str() == access_point.ssid.as_ref() {
                                        self.new_connection = None;
                                        self.show_visible_networks = false;
                                    }
                                }
                            }
                            _ => {}
                        }

                        self.nm_state = state;
                        self.vpn_active = self.nm_state.active_conns.iter().any(|c| matches!(c, cosmic_settings_network_manager_subscription::ActiveConnectionInfo::Vpn { .. }));
                    }
                    NmEvent::Devices | NmEvent::WirelessAccessPoints | NmEvent::ActiveConns | NmEvent::WiFiCredentials { .. } => {
                        if let Some(ref sender) = self.nm_sender {
                            let _ = sender.unbounded_send(NmRequest::Reload);
                        }
                    }
                    NmEvent::WiFiEnabled(enabled) => {
                        self.nm_state.wifi_enabled = enabled;
                        if let Some(ref sender) = self.nm_sender {
                            let _ = sender.unbounded_send(NmRequest::Reload);
                        }
                    }
                }
            }

            // ── D-Bus: Sound/Audio ───────────────────────────────────────────
            Message::Sound(msg) => {
                self.sound_available = true;
                // Trigger cosmic-osd via wpctl which emits the correct DBus/Pulse events on apply (respecting debounce)
                match &msg {
                    cosmic_settings_sound_subscription::Message::SinkVolumeApply(_) => {
                        let _ = std::process::Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{}%", self.volume)]).spawn();
                    }
                    cosmic_settings_sound_subscription::Message::SourceVolumeApply(_) => {
                        let _ = std::process::Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", &format!("{}%", self.sound.source_volume)]).spawn();
                    }
                    _ => {}
                }
                let task = self.sound.update(msg).map(|m| cosmic::Action::App(Message::Sound(m)));
                self.volume = self.sound.sink_volume;
                self.global_mute = self.sound.sink_mute;
                return task;
            }

            // ── D-Bus: Bluetooth (Bluer) ─────────────────────────────────────
            Message::BluetoothEvent(e) => {
                match e {
                    BluerEvent::RequestResponse {
                        req: _,
                        state,
                        err_msg,
                    } => {
                        if let Some(err_msg) = err_msg {
                            tracing::error!("bluetooth request error: {err_msg}");
                        }
                        self.bluer_state = state;
                        self.bluetooth_available = true;
                    }
                    BluerEvent::Init { sender, state } => {
                        self.bluer_sender.replace(sender);
                        self.bluer_state = state;
                        self.bluetooth_available = true;
                    }
                    BluerEvent::DevicesChanged { state } => {
                        self.bluer_state = state;
                        self.bluetooth_available = true;
                    }
                    BluerEvent::Finished => {
                        tracing::error!("bluetooth subscription finished");
                        self.bluetooth_available = false;
                    }
                    BluerEvent::AgentEvent(event) => match event {
                        BluerAgentEvent::RequestConfirmation(d, code, tx) => {
                            self.request_confirmation.replace((d, code, tx));
                        }
                        _ => {
                            // Other agent events (PinCode display, Passkey display) can be handled later
                            tracing::info!("AgentEvent received but not yet implemented in UI: {:?}", event);
                        }
                    },
                }
            }
            Message::BluetoothRequest(r) => {
                // Optimistic UI updates
                match &r {
                    BluerRequest::SetBluetoothEnabled(enabled) => {
                        self.bluer_state.bluetooth_enabled = *enabled;
                    }
                    BluerRequest::ConnectDevice(add) => {
                        if let Some(d) = self.bluer_state.devices.iter_mut().find(|d| d.address == *add) {
                            d.status = bluetooth::BluerDeviceStatus::Connecting;
                        }
                    }
                    BluerRequest::DisconnectDevice(add) => {
                        if let Some(d) = self.bluer_state.devices.iter_mut().find(|d| d.address == *add) {
                            d.status = bluetooth::BluerDeviceStatus::Disconnecting;
                        }
                    }
                    BluerRequest::PairDevice(add) => {
                        if let Some(d) = self.bluer_state.devices.iter_mut().find(|d| d.address == *add) {
                            d.status = bluetooth::BluerDeviceStatus::Pairing;
                        }
                    }
                    _ => {}
                }
                if let Some(tx) = self.bluer_sender.clone() {
                    tokio::spawn(async move {
                        let _ = tx.try_send(r);
                    });
                }
            }
            Message::ConfirmPairing => {
                if let Some((_, _, tx)) = self.request_confirmation.take() {
                    tokio::spawn(async move {
                        let _ = tx.try_send(true);
                    });
                }
            }
            Message::CancelPairing => {
                if let Some((_, _, tx)) = self.request_confirmation.take() {
                    tokio::spawn(async move {
                        let _ = tx.try_send(false);
                    });
                }
            }
            Message::ToggleVisibleDevices(enabled) => {
                self.show_visible_devices = enabled;
            }

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
                // Close the popup first, and flag that we are waiting to screenshot
                self.pending_screenshot = true;
                
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                } else {
                    return Task::perform(async {}, |_| cosmic::Action::App(Message::ExecuteScreenshot));
                }
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
                    async { let _ = crate::subscriptions::power::power_off().await; },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }
            Message::Suspend => {
                return Task::perform(
                    async { let _ = crate::subscriptions::power::suspend().await; },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }
            Message::OpenSettings(page) => {
                return Task::perform(
                    async move {
                        let mut cmd = tokio::process::Command::new("cosmic-settings");
                        if let Some(p) = page {
                            cmd.arg(p);
                        }
                        let _ = cmd.spawn();
                    },
                    |_| cosmic::Action::App(Message::Navigate(AppView::Main)),
                );
            }
            // ── Network Manager Actions ─────────────────────────────
            Message::ToggleVisibleNetworks => {
                self.show_visible_networks = !self.show_visible_networks;
            }
            Message::SelectWirelessAccessPoint(access_point) => {
                let Some(tx) = self.nm_sender.as_ref() else {
                    return Task::none();
                };

                if matches!(access_point.network_type, nm_sub::available_wifi::NetworkType::Open) {
                    if let Err(err) =
                        tx.unbounded_send(nm_sub::Request::SelectAccessPoint(
                            access_point.ssid.clone(),
                            access_point.network_type,
                            self.secret_tx.clone(),
                            None,
                        ))
                    {
                        if err.is_disconnected() {
                            return crate::network::system_conn().map(cosmic::Action::App);
                        }

                        tracing::error!("{err:?}");
                    }
                    self.new_connection = Some(NewConnectionState::Waiting(access_point));
                } else {
                    if self
                        .nm_state
                        .known_access_points
                        .contains(&access_point)
                    {
                        if let Err(err) =
                            tx.unbounded_send(nm_sub::Request::SelectAccessPoint(
                                access_point.ssid.clone(),
                                access_point.network_type,
                                self.secret_tx.clone(),
                                None,
                            ))
                        {
                            if err.is_disconnected() {
                                return crate::network::system_conn().map(cosmic::Action::App);
                            }

                            tracing::error!("{err:?}");
                        }
                    }
                    self.new_connection = Some(NewConnectionState::EnterPassword {
                        access_point,
                        description: None,
                        identity: String::new(),
                        password: String::new().into(),
                        password_hidden: true,
                    });
                }
            }
            Message::CancelNewConnection => {
                self.new_connection = None;
            }
            Message::Connect(ssid, hw_address) => {
                let mut network_type = nm_sub::available_wifi::NetworkType::Open;
                let tx = if let Some(tx) = self.nm_sender.as_ref() {
                    if let Some(ap) = self
                        .nm_state
                        .known_access_points
                        .iter_mut()
                        .find(|c| c.ssid == ssid && c.hw_address == hw_address)
                    {
                        network_type = ap.network_type;
                        ap.working = true;
                    }
                    tx
                } else {
                    return Task::none();
                };
                if let Err(err) = tx.unbounded_send(nm_sub::Request::SelectAccessPoint(
                    ssid,
                    network_type,
                    self.secret_tx.clone(),
                    None,
                )) {
                    if err.is_disconnected() {
                        return crate::network::system_conn().map(cosmic::Action::App);
                    }
                    tracing::error!("{err:?}");
                }
            }
            Message::ConnectWithPassword => {
                let Some(tx) = self.nm_sender.as_ref() else {
                    return Task::none();
                };

                if let Some(NewConnectionState::EnterPassword {
                    password,
                    access_point,
                    identity,
                    ..
                }) = self.new_connection.take()
                {
                    let is_enterprise: bool = matches!(access_point.network_type, nm_sub::available_wifi::NetworkType::EAP);

                    if let Err(err) = tx.unbounded_send(nm_sub::Request::Authenticate {
                        ssid: access_point.ssid.to_string(),
                        identity: is_enterprise.then(|| identity.clone()),
                        password,
                        secret_tx: self.secret_tx.clone(),
                        interface: None,
                    }) {
                        if err.is_disconnected() {
                            return crate::network::system_conn().map(cosmic::Action::App);
                        }
                        tracing::error!("Failed to authenticate with network manager");
                    }
                    self.new_connection
                        .replace(NewConnectionState::Waiting(access_point));
                }
            }
            Message::Disconnect(ssid, hw_address) => {
                self.new_connection = None;
                let tx = if let Some(tx) = self.nm_sender.as_ref() {
                    if let Some(ActiveConnectionInfo::WiFi { state, .. }) =
                        self.nm_state.active_conns.iter_mut().find(|c| {
                            let c_hw_address = match c {
                                ActiveConnectionInfo::Wired { hw_address, .. }
                                | ActiveConnectionInfo::WiFi { hw_address, .. } => {
                                    HwAddress::from_str(hw_address).unwrap_or_default()
                                }
                                ActiveConnectionInfo::Vpn { .. } => HwAddress::default(),
                            };
                            c.name().as_str() == ssid.as_ref() && c_hw_address == hw_address
                        })
                    {
                        *state = cosmic_dbus_networkmanager::interface::enums::ActiveConnectionState::Deactivating;
                    }
                    tx
                } else {
                    return Task::none();
                };
                if let Err(err) = tx.unbounded_send(nm_sub::Request::Disconnect(ssid)) {
                    if err.is_disconnected() {
                        return crate::network::system_conn().map(cosmic::Action::App);
                    }
                    tracing::error!("{err:?}");
                }
            }
            Message::PasswordUpdate(pwd) => {
                if let Some(NewConnectionState::EnterPassword { password, .. }) = &mut self.new_connection {
                    *password = pwd;
                }
            }
            Message::IdentityUpdate(identity) => {
                if let Some(NewConnectionState::EnterPassword { identity: current, .. }) = &mut self.new_connection {
                    *current = identity;
                }
            }
            Message::TogglePasswordVisibility => {
                if let Some(NewConnectionState::EnterPassword { password_hidden, .. }) = &mut self.new_connection {
                    *password_hidden = !*password_hidden;
                }
            }
            Message::SecretAgent(agent_event) => match agent_event {
                nm_secret_agent::Event::RequestSecret {
                    uuid,
                    name,
                    description,
                    previous,
                    tx,
                    ..
                } => {
                    if let Some(state) = self.new_connection.as_mut() {
                        match state {
                            NewConnectionState::EnterPassword { access_point, .. }
                            | NewConnectionState::Waiting(access_point)
                            | NewConnectionState::Failure(access_point) => {
                                if self
                                    .ssid_to_uuid
                                    .get(access_point.ssid.as_ref())
                                    .is_some_and(|ap_uuid| ap_uuid.as_ref() == uuid.as_str())
                                {
                                    *state = NewConnectionState::EnterPassword {
                                        access_point: access_point.clone(),
                                        description,
                                        identity: String::new(),
                                        password: String::new().into(),
                                        password_hidden: true,
                                    }
                                }
                            }
                        }
                    } else if self.known_vpns.contains_key(uuid.as_str()) {
                        self.requested_vpn = Some(RequestedVpn {
                            name,
                            uuid: uuid.into(),
                            description,
                            password: previous,
                            password_hidden: true,
                            tx,
                        });
                    }
                }
                nm_secret_agent::Event::CancelGetSecrets { .. } => {
                    self.new_connection = None;
                    self.requested_vpn = None;
                }
                nm_secret_agent::Event::Failed(error) => {
                    tracing::error!("Error from secret agent: {error:?}");
                }
            },
            Message::NetworkManagerConnect(conn) => {
                self.conn = Some(conn);
            }
            Message::ConnectionSettings(settings) => {
                self.ssid_to_uuid = settings;
            }
            Message::KnownConnections(known) => {
                self.known_vpns = known;
            }
            Message::ResetFailedKnownSsid(ssid, hw_address) => {
                let ap = if let Some(pos) = self
                    .nm_state
                    .known_access_points
                    .iter()
                    .position(|ap| ap.ssid.as_ref() == ssid.as_str() && ap.hw_address == hw_address)
                {
                    self.nm_state.known_access_points.remove(pos)
                } else if let Some((pos, ap)) = self
                    .nm_state
                    .active_conns
                    .iter()
                    .position(|conn| {
                        let c_hw_address = match conn {
                            ActiveConnectionInfo::Wired { hw_address, .. }
                            | ActiveConnectionInfo::WiFi { hw_address, .. } => {
                                HwAddress::from_str(hw_address).unwrap_or_default()
                            }
                            ActiveConnectionInfo::Vpn { .. } => HwAddress::default(),
                        };
                        conn.name() == ssid && c_hw_address == hw_address
                    })
                    .zip(
                        self.nm_state
                            .wireless_access_points
                            .iter()
                            .find(|ap| {
                                ap.ssid.as_ref() == ssid.as_str() && ap.hw_address == hw_address
                            }),
                    )
                {
                    self.nm_state.active_conns.remove(pos);
                    ap.clone()
                } else {
                    tracing::warn!("Failed to find known access point with ssid: {}", ssid);
                    return Task::none();
                };
                if let Some(tx) = self.nm_sender.as_ref() {
                    if let Err(err) =
                        tx.unbounded_send(nm_sub::Request::Forget(ssid.into()))
                    {
                        if err.is_disconnected() {
                            return crate::network::system_conn().map(cosmic::Action::App);
                        }

                        tracing::error!("{err:?}");
                    }
                    self.show_visible_networks = true;
                    return self.update(Message::SelectWirelessAccessPoint(ap));
                }
            }
            Message::ActivateVpn(uuid) => {
                if let Some((tx, conn)) = self.nm_sender.clone().zip(self.conn.clone()) {
                    return crate::network::connect_vpn(conn, tx, uuid).map(cosmic::Action::App);
                }
            }
            Message::DeactivateVpn(uuid) => {
                let name = if let Some(connection) = self.known_vpns.get(uuid.as_ref()) {
                    match connection {
                        crate::network::ConnectionSettings::Vpn(connection) => connection.id.clone(),
                        crate::network::ConnectionSettings::Wireguard { id } => id.clone(),
                    }
                } else {
                    return Task::none();
                };
                if let Some(tx) = self.nm_sender.as_ref() {
                    if let Err(err) = tx.unbounded_send(nm_sub::Request::Deactivate(name.into()))
                    {
                        if err.is_disconnected() {
                            return crate::network::system_conn().map(cosmic::Action::App);
                        }
                        tracing::error!("{err:?}");
                    }
                }
            }
            Message::ToggleVpnPasswordVisibility => {
                if let Some(requested_vpn) = self.requested_vpn.as_mut() {
                    requested_vpn.password_hidden = !requested_vpn.password_hidden;
                }
            }
            Message::ConnectVPNWithPassword => {
                if let Some(RequestedVpn { password, tx, .. }) = self.requested_vpn.take() {
                    return Task::future(async move {
                        let mut guard = tx.lock().await;
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(password);
                        }
                        Message::Refresh
                    }).map(cosmic::Action::App);
                }
            }
            Message::VPNPasswordUpdate(pwd) => {
                if let Some(requested_vpn) = self.requested_vpn.as_mut() {
                    requested_vpn.password = pwd;
                }
            }
            Message::CancelVPNConnection => {
                self.requested_vpn = None;
            }
            Message::UpdateState(state) => {
                self.nm_state = state;
            }
            Message::UpdateDevices(devices) => {
                self.nm_devices = devices.into_iter().map(Arc::new).collect();
            }
            Message::Refresh => {
                if let Some(conn) = self.conn.clone() {
                    return Task::batch(vec![
                        crate::network::update_state(conn.clone()),
                        crate::network::update_devices(conn.clone()),
                        crate::network::load_vpns(conn),
                    ]).map(cosmic::Action::App);
                }
            }
            Message::Error(err) => {
                tracing::error!("Network applet error: {}", err);
            }

            // ── Popup lifecycle ──────────────────────────────────────
            Message::TogglePopup => {
                // Reset to main view when toggling popup
                self.current_view = AppView::Main;

                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else if let Some(main_id) = self.core.main_window_id() {
                    let new_id = window::Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        main_id,
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
                        
                    // Make sure bluetooth discovery is enabled while popup is open
                    bluetooth::set_discovery(true);
                    
                    get_popup(popup_settings)
                } else {
                    bluetooth::set_discovery(false);
                    Task::none()
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    bluetooth::set_discovery(false);
                    self.popup = None;
                    self.current_view = AppView::Main;
                }
                if self.pending_screenshot {
                    self.pending_screenshot = false;
                    return Task::perform(async {}, |_| cosmic::Action::App(Message::ExecuteScreenshot));
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

// ── Boilerplate getters ──────────────────────────────────────────────────────
fn get_sound_watch(_: &&str) -> impl futures::Stream<Item = cosmic_settings_sound_subscription::Message> + cosmic::iced_futures::MaybeSend + 'static {
    cosmic_settings_sound_subscription::watch()
}
