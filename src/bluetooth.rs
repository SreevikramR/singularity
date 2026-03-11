// SPDX-License-Identifier: GPL-3.0-only
//
// Bluetooth subsystem using bluer (BlueZ D-Bus bindings).
// Ported from cosmic-applet-bluetooth.

use rustc_hash::FxHashMap;

use std::{
    fmt::Debug,
    hash::Hash,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bluer::{
    Adapter, AdapterProperty, Address, Session, Uuid,
    agent::{Agent, AgentHandle},
};

use cosmic::{
    iced::{
        self, Subscription,
        futures::{SinkExt, StreamExt},
    },
    iced_futures::stream,
};

use futures::{FutureExt, stream::FuturesUnordered};
use tokio::{
    spawn,
    sync::{
        Mutex, RwLock,
        mpsc::{Receiver, Sender, channel},
    },
    task::JoinHandle,
};

static TICK: LazyLock<RwLock<Duration>> = LazyLock::new(|| RwLock::new(Duration::from_secs(10)));
static DISCOVERY: AtomicBool = AtomicBool::new(false);

pub fn set_discovery(v: bool) {
    DISCOVERY.store(v, Ordering::Relaxed);
}

pub async fn set_tick(duration: Duration) {
    let mut guard = TICK.write().await;
    *guard = duration;
}

async fn tick(interval: &mut tokio::time::Interval) {
    let guard = TICK.read().await;
    if *guard != interval.period() {
        *interval = tokio::time::interval(*guard);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    }
    interval.tick().await;
}

// Device icon mapping from BlueZ device types
const DEFAULT_DEVICE_ICON: &str = "bluetooth-symbolic";

fn device_type_to_icon(device_type: &str) -> &'static str {
    match device_type {
        "computer" => "laptop-symbolic",
        "phone" => "smartphone-symbolic",
        "network-wireless" => "network-wireless-symbolic",
        "audio-headset" => "audio-headset-symbolic",
        "audio-headphones" => "audio-headphones-symbolic",
        "camera-video" => "camera-video-symbolic",
        "audio-card" => "audio-card-symbolic",
        "input-gaming" => "input-gaming-symbolic",
        "input-keyboard" => "input-keyboard-symbolic",
        "input-tablet" => "input-tablet-symbolic",
        "input-mouse" => "input-mouse-symbolic",
        "printer" => "printer-network-symbolic",
        "camera-photo" => "camera-photo-symbolic",
        _ => DEFAULT_DEVICE_ICON,
    }
}

// rfkill PATH helper — some distros only have rfkill in /usr/sbin
fn rfkill_path_var() -> std::ffi::OsString {
    let mut path = std::env::var_os("PATH").unwrap_or_default();
    path.push(":/usr/sbin");
    path
}

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BluerDeviceStatus {
    Connected,
    Connecting,
    Paired,
    Pairing,
    Disconnected,
    Disconnecting,
}

#[derive(Debug, Clone)]
pub struct BluerDevice {
    pub name: String,
    pub icon: &'static str,
    pub address: Address,
    pub status: BluerDeviceStatus,
    pub battery_percent: Option<u8>,
    pub is_paired: bool,
    pub is_trusted: bool,
}

impl Eq for BluerDevice {}

impl Ord for BluerDevice {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.status.cmp(&other.status) {
            std::cmp::Ordering::Equal => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
            o => o,
        }
    }
}

impl PartialOrd for BluerDevice {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BluerDevice {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.address == other.address
    }
}

impl BluerDevice {
    pub async fn from_device(device: &bluer::Device) -> Self {
        let (mut name, is_paired, is_trusted, is_connected, battery_percent, icon) = futures::join!(
            device
                .name()
                .map(|res| res.ok().flatten().unwrap_or(device.address().to_string())),
            device.is_paired().map(Result::unwrap_or_default),
            device.is_trusted().map(Result::unwrap_or_default),
            device.is_connected().map(Result::unwrap_or_default),
            device.battery_percentage().map(|res| res.ok().flatten()),
            device
                .icon()
                .map(|res| device_type_to_icon(&res.ok().flatten().unwrap_or_default()))
        );

        if name.is_empty() {
            name = device.address().to_string();
        }

        let status = if is_connected {
            BluerDeviceStatus::Connected
        } else if is_paired {
            BluerDeviceStatus::Paired
        } else {
            BluerDeviceStatus::Disconnected
        };

        Self {
            name,
            icon,
            address: device.address(),
            status,
            battery_percent,
            is_paired,
            is_trusted,
        }
    }

    pub fn paired_and_trusted(&self) -> bool {
        self.is_paired && self.is_trusted
    }

    pub fn is_known_device_type(&self) -> bool {
        self.icon != DEFAULT_DEVICE_ICON
    }

    pub fn has_name(&self) -> bool {
        self.name != self.address.to_string()
    }
}

// ── Request / Event types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum BluerRequest {
    SetBluetoothEnabled(bool),
    PairDevice(Address),
    ConnectDevice(Address),
    DisconnectDevice(Address),
    CancelConnect(Address),
}

#[derive(Debug, Clone)]
pub enum BluerEvent {
    RequestResponse {
        req: BluerRequest,
        state: BluerState,
        err_msg: Option<String>,
    },
    Init {
        sender: Sender<BluerRequest>,
        state: BluerState,
    },
    DevicesChanged {
        state: BluerState,
    },
    AgentEvent(BluerAgentEvent),
    Finished,
}

#[derive(Debug, Clone, Default)]
pub struct BluerState {
    pub devices: Vec<BluerDevice>,
    pub bluetooth_enabled: bool,
}

#[derive(Debug, Clone)]
pub enum BluerAgentEvent {
    DisplayPinCode(BluerDevice, String),
    DisplayPasskey(BluerDevice, String),
    RequestPinCode(BluerDevice),
    RequestPasskey(BluerDevice),
    RequestConfirmation(BluerDevice, String, Sender<bool>),
    RequestDeviceAuthorization(BluerDevice, Sender<bool>),
    RequestServiceAuthorization(BluerDevice, Uuid, Sender<bool>),
}

// ── Internal session types ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum BluerSessionEvent {
    RequestResponse {
        req: BluerRequest,
        state: BluerState,
        err_msg: Option<String>,
    },
    ChangesProcessed(BluerState),
    AgentEvent(BluerAgentEvent),
}

struct BluerSessionState {
    _session: Session,
    _agent_handle: AgentHandle,
    adapters: Vec<Adapter>,
    rx: Option<Receiver<BluerSessionEvent>>,
    req_tx: Sender<BluerRequest>,
    wake_up_discover_tx: Sender<()>,
    wake_up_discover_rx: Option<Receiver<()>>,
    tx: Sender<BluerSessionEvent>,
    active_requests: Arc<Mutex<FxHashMap<BluerRequest, JoinHandle<anyhow::Result<()>>>>>,
}

impl BluerSessionState {
    async fn new(session: Session) -> anyhow::Result<Self> {
        let mut adapters = Vec::new();
        if let Ok(names) = session.adapter_names().await {
            for name in names {
                if let Ok(adapter) = session.adapter(&name) {
                    adapters.push(adapter);
                }
            }
        }
        if adapters.is_empty() {
            if let Ok(adapter) = session.default_adapter().await {
                adapters.push(adapter);
            } else {
                return Err(anyhow::anyhow!("No bluetooth adapters found"));
            }
        }
        
        // We'll use the first adapter for the agent for simplicity, but we'll monitor all adapters
        let agent_adapter = adapters[0].clone();
        
        let (tx, rx) = channel(100);
        let (req_tx, req_rx) = channel(100);

        // Create agent clones for each callback
        let tx_clone_1 = tx.clone();
        let tx_clone_2 = tx.clone();
        let tx_clone_3 = tx.clone();
        let tx_clone_4 = tx.clone();
        let tx_clone_5 = tx.clone();
        let tx_clone_6 = tx.clone();
        let tx_clone_7 = tx.clone();
        let adapter_clone_1 = agent_adapter.clone();
        let adapter_clone_2 = agent_adapter.clone();
        let adapter_clone_3 = agent_adapter.clone();
        let adapter_clone_4 = agent_adapter.clone();
        let adapter_clone_5 = agent_adapter.clone();
        let adapter_clone_6 = agent_adapter.clone();
        let adapter_clone_7 = agent_adapter.clone();

        let _agent = Agent {
            request_default: false,
            request_pin_code: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_1.clone();
                let tx_clone = tx_clone_1.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::RequestPinCode(
                                BluerDevice::from_device(&device).await,
                            ),
                        ))
                        .await;
                    let pin_code = fastrand::u32(0..999999);
                    Ok(format!("{pin_code:06}"))
                })
            })),
            display_pin_code: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_2.clone();
                let tx_clone = tx_clone_2.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::DisplayPinCode(
                                BluerDevice::from_device(&device).await,
                                req.pincode,
                            ),
                        ))
                        .await;
                    Ok(())
                })
            })),
            request_passkey: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_3.clone();
                let tx_clone = tx_clone_3.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::RequestPasskey(
                                BluerDevice::from_device(&device).await,
                            ),
                        ))
                        .await;
                    let pin_code = fastrand::u32(0..999999);
                    Ok(pin_code)
                })
            })),
            display_passkey: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_4.clone();
                let tx_clone = tx_clone_4.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::DisplayPasskey(
                                BluerDevice::from_device(&device).await,
                                format!("{:06}", req.passkey),
                            ),
                        ))
                        .await;
                    Ok(())
                })
            })),
            request_confirmation: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_5.clone();
                let tx_clone = tx_clone_5.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let (tx, mut rx) = channel(1);
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::RequestConfirmation(
                                BluerDevice::from_device(&device).await,
                                format!("{:06}", req.passkey),
                                tx,
                            ),
                        ))
                        .await;
                    match rx.recv().await {
                        Some(true) => Ok(()),
                        _ => Err(bluer::agent::ReqError::Rejected),
                    }
                })
            })),
            request_authorization: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_6.clone();
                let tx_clone = tx_clone_6.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let (tx, mut rx) = channel(1);
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::RequestDeviceAuthorization(
                                BluerDevice::from_device(&device).await,
                                tx,
                            ),
                        ))
                        .await;
                    match rx.recv().await {
                        Some(true) => Ok(()),
                        _ => Err(bluer::agent::ReqError::Rejected),
                    }
                })
            })),
            authorize_service: Some(Box::new(move |req| {
                let agent_clone = adapter_clone_7.clone();
                let tx_clone = tx_clone_7.clone();
                Box::pin(async move {
                    let Ok(device) = agent_clone.device(req.device) else {
                        return Err(bluer::agent::ReqError::Rejected);
                    };
                    let (tx, mut rx) = channel(1);
                    let _ = tx_clone
                        .send(BluerSessionEvent::AgentEvent(
                            BluerAgentEvent::RequestServiceAuthorization(
                                BluerDevice::from_device(&device).await,
                                req.service,
                                tx,
                            ),
                        ))
                        .await;
                    match rx.recv().await {
                        Some(true) => Ok(()),
                        _ => Err(bluer::agent::ReqError::Rejected),
                    }
                })
            })),
            _non_exhaustive: (),
        };

        let _agent_handle = session.register_agent(_agent).await?;
        let (wake_up_discover_tx, wake_up_discover_rx) = channel(10);

        let mut self_ = Self {
            _agent_handle,
            _session: session,
            adapters,
            rx: Some(rx),
            req_tx,
            wake_up_discover_rx: Some(wake_up_discover_rx),
            wake_up_discover_tx,
            tx,
            active_requests: Default::default(),
        };

        self_.process_requests(req_rx);
        self_.process_changes();
        self_.listen_bluetooth_power_changes();

        Ok(self_)
    }

    fn listen_bluetooth_power_changes(&self) {
        let tx = self.tx.clone();
        let req_tx = self.req_tx.clone();
        let adapters_clone = self.adapters.clone();
        let wake_up_discover_tx = self.wake_up_discover_tx.clone();
        let _handle: JoinHandle<anyhow::Result<()>> = spawn(async move {
            let mut status = false;
            for a in &adapters_clone {
                if a.is_powered().await.unwrap_or_default() {
                    status = true;
                    break;
                }
            }
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut devices = Vec::new();
            loop {
                tick(&mut interval).await;
                
                let mut new_status = false;
                for a in &adapters_clone {
                    if a.is_powered().await.unwrap_or_default() {
                        new_status = true;
                        break;
                    }
                }
                
                devices.clear();
                for adapter_clone in &adapters_clone {
                    devices = build_device_list(devices, adapter_clone).await;
                }
                
                if new_status != status {
                    status = new_status;
                    let state = BluerState {
                        devices: devices.clone(),
                        bluetooth_enabled: status,
                    };
                    if state.bluetooth_enabled {
                        for d in &state.devices {
                            if d.paired_and_trusted() {
                                let _ = req_tx.send(BluerRequest::ConnectDevice(d.address)).await;
                            }
                        }
                    }
                    let _ = wake_up_discover_tx.send(()).await;
                    let _ = tx
                        .send(BluerSessionEvent::ChangesProcessed(state))
                        .await;
                }
            }
        });
    }

    fn process_changes(&mut self) {
        let req_tx = self.req_tx.clone();
        let tx = self.tx.clone();
        let Some(mut wake_up) = self.wake_up_discover_rx.take() else {
            tracing::error!("Failed to take wake up channel");
            return;
        };
        let adapters_clone = self.adapters.clone();

        spawn(async move {
            if adapters_clone.is_empty() { return; }
            let adapter_clone = adapters_clone[0].clone(); // Only process discovery events for the main adapter for simplicity, as scanning runs across all generally in most UI, but for tracking change streams one is usually sufficient.
            
            let mut is_powered = false;
            for a in &adapters_clone {
                if a.is_powered().await.unwrap_or_default() {
                    is_powered = true;
                    break;
                }
            }
            let mut devices: Vec<BluerDevice> = Vec::new();
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                let wakeup_fut = wake_up.recv();

                let listener_fut = async {
                    if DISCOVERY.load(Ordering::SeqCst) || devices.is_empty() {
                        let mut interval = tokio::time::interval(Duration::from_secs(10));
                        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                        let Ok(mut change_stream) =
                            adapter_clone.discover_devices_with_changes().await
                        else {
                            tick(&mut interval).await;
                            return;
                        };

                        loop {
                            let Some(adapter_event) = change_stream.next().await else {
                                break;
                            };
                            match adapter_event {
                                bluer::AdapterEvent::PropertyChanged(AdapterProperty::Powered(
                                    v,
                                )) => {
                                    is_powered = v;
                                }
                                e => {
                                    match e {
                                        bluer::AdapterEvent::DeviceAdded(address)
                                            if !devices.iter().any(|d| d.address == address) =>
                                        {
                                            devices =
                                                build_device_list(Vec::new(), &adapter_clone).await;
                                            for d in devices.iter().filter(|d| {
                                                d.paired_and_trusted()
                                                    && !matches!(
                                                        d.status,
                                                        BluerDeviceStatus::Connected
                                                    )
                                            }) {
                                                let _ = req_tx
                                                    .send(BluerRequest::ConnectDevice(d.address))
                                                    .await;
                                            }
                                        }
                                        bluer::AdapterEvent::DeviceRemoved(address)
                                            if devices.iter().any(|d| d.address == address) =>
                                        {
                                            devices.retain(|d| d.address != address);
                                        }
                                        bluer::AdapterEvent::PropertyChanged(p) => {
                                            tracing::info!("property change ignored {p:?}");
                                            interval.tick().await;
                                            continue;
                                        }
                                        bluer::AdapterEvent::DeviceAdded(address)
                                        | bluer::AdapterEvent::DeviceRemoved(address) => {
                                            tracing::info!(
                                                "device change already handled {address}"
                                            );
                                            interval.tick().await;
                                            continue;
                                        }
                                    }
                                }
                            }

                            let _ = tx
                                .send(BluerSessionEvent::ChangesProcessed(BluerState {
                                    devices: devices.clone(),
                                    bluetooth_enabled: is_powered,
                                }))
                                .await;

                            interval.tick().await;
                            if !DISCOVERY.load(Ordering::SeqCst) && !devices.is_empty() {
                                break;
                            }
                        }
                    } else {
                        loop {
                            if DISCOVERY.load(Ordering::SeqCst) || devices.is_empty() {
                                break;
                            }
                            interval.tick().await;
                        }
                    }
                };

                futures::pin_mut!(listener_fut);
                futures::pin_mut!(wakeup_fut);

                futures::future::select(listener_fut, wakeup_fut).await;
            }
        });
    }

    fn process_requests(&mut self, request_rx: Receiver<BluerRequest>) {
        let active_requests = self.active_requests.clone();
        let adapters = self.adapters.clone();
        let tx = self.tx.clone();
        let wake_up_tx = self.wake_up_discover_tx.clone();

        let _handle: JoinHandle<anyhow::Result<()>> = spawn(async move {
            let mut request_rx = request_rx;

            while let Some(req) = request_rx.recv().await {
                let req_clone = req.clone();
                let req_clone_2 = req.clone();
                let active_requests_clone = active_requests.clone();
                let tx_clone = tx.clone();
                let adapters_clone = adapters.clone();
                let wake_up_tx = wake_up_tx.clone();

                let handle = spawn(async move {
                    let mut err_msg = None;
                    match &req_clone {
                        BluerRequest::SetBluetoothEnabled(enabled) => {
                            for adapter_clone in &adapters_clone {
                                if let Err(e) = adapter_clone.set_powered(*enabled).await {
                                    tracing::error!("Failed to power bluetooth adapter: {e:?}");
                                }

                                // Use rfkill to persist state across reboots
                                let name = adapter_clone.name();
                                if let Some(id) = tokio::process::Command::new("rfkill")
                                    .env("PATH", rfkill_path_var())
                                    .arg("list")
                                    .arg("-n")
                                    .arg("--output")
                                    .arg("ID,DEVICE")
                                    .output()
                                    .await
                                    .ok()
                                    .and_then(|o| {
                                        let lines = String::from_utf8(o.stdout).ok()?;
                                        lines.split('\n').find_map(|row| {
                                            let (id, cname) = row.trim().split_once(' ')?;
                                            (name == cname).then_some(id.to_string())
                                        })
                                    })
                                {
                                    if let Err(err) = tokio::process::Command::new("rfkill")
                                        .env("PATH", rfkill_path_var())
                                        .arg(if *enabled { "unblock" } else { "block" })
                                        .arg(id)
                                        .output()
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to set bluetooth state via rfkill: {err:?}"
                                        );
                                    }
                                }
                            }

                            if *enabled {
                                let _ = wake_up_tx.send(()).await;
                            }
                        }
                        BluerRequest::PairDevice(address) => {
                            let mut handled = false;
                            for adapter_clone in &adapters_clone {
                                if let Ok(device) = adapter_clone.device(*address) {
                                    handled = true;
                                    if let Err(err) = device.pair().await {
                                        err_msg = Some(err.to_string());
                                    } else if let Err(err) = device.set_trusted(true).await {
                                        tracing::error!(?err, "Failed to trust device.");
                                    }
                                    break;
                                }
                            }
                            if !handled { err_msg = Some("Device not found on any adapter".into()); }
                        }
                        BluerRequest::ConnectDevice(address) => {
                            let mut handled = false;
                            for adapter_clone in &adapters_clone {
                                if let Ok(device) = adapter_clone.device(*address) {
                                    handled = true;
                                    if let Err(err) = device.connect().await {
                                        err_msg = Some(err.to_string());
                                    } else if let Err(err) = device.set_trusted(true).await {
                                        tracing::error!(?err, "Failed to trust device.");
                                    }
                                    break;
                                }
                            }
                            if !handled { err_msg = Some("Device not found on any adapter".into()); }
                        }
                        BluerRequest::DisconnectDevice(address) => {
                            let mut handled = false;
                            for adapter_clone in &adapters_clone {
                                if let Ok(device) = adapter_clone.device(*address) {
                                    handled = true;
                                    if let Err(err) = device.disconnect().await {
                                        err_msg = Some(err.to_string());
                                    }
                                    break;
                                }
                            }
                            if !handled { err_msg = Some("Device not found on any adapter".into()); }
                        }
                        BluerRequest::CancelConnect(_) => {
                            if let Some(handle) =
                                active_requests_clone.lock().await.get(&req_clone)
                            {
                                handle.abort();
                            } else {
                                err_msg =
                                    Some("No active connection request found".to_string());
                            }
                        }
                    }

                    // For the response state, just query the primary adapter for now
                    let st = if adapters_clone.is_empty() {
                        BluerState::default()
                    } else {
                        bluer_state(&adapters_clone[0]).await
                    };

                    let _ = tx_clone
                        .send(BluerSessionEvent::RequestResponse {
                            req: req_clone,
                            state: st,
                            err_msg,
                        })
                        .await;

                    active_requests_clone.lock().await.remove(&req_clone_2);

                    Ok(())
                });

                active_requests.lock().await.insert(req, handle);
            }
            Ok(())
        });
    }
}

// ── Public subscription ──────────────────────────────────────────────────────

pub fn bluetooth_subscription<I: 'static + Hash + Copy + Send + Sync + Debug>(
    id: I,
) -> iced::Subscription<BluerEvent> {
    Subscription::run_with(
        id,
        |_id| {
            stream::channel(50, move |mut output: futures::channel::mpsc::Sender<BluerEvent>| async move {
            let mut retry_count = 0u32;

            // Initialize connection with exponential backoff
            let mut session_state = loop {
                if let Ok(session) = Session::new().await {
                    if let Ok(state) = BluerSessionState::new(session).await {
                        break state;
                    }
                }

                retry_count = retry_count.saturating_add(1);
                tokio::time::sleep(Duration::from_millis(
                    2_u64.saturating_pow(retry_count).min(68719476734),
                ))
                .await;
            };

            let state = if session_state.adapters.is_empty() {
                BluerState::default()
            } else {
                bluer_state(&session_state.adapters[0]).await
            };

            // Auto-reconnect paired and trusted devices
            if state.bluetooth_enabled {
                for d in &state.devices {
                    if d.paired_and_trusted()
                        && !matches!(d.status, BluerDeviceStatus::Connected)
                    {
                        let _ = session_state
                            .req_tx
                            .send(BluerRequest::ConnectDevice(d.address))
                            .await;
                    }
                }
            }

            let _ = output
                .send(BluerEvent::Init {
                    sender: session_state.req_tx.clone(),
                    state: state.clone(),
                })
                .await;

            let mut event_handler = async |event| {
                let message = match event {
                    BluerSessionEvent::ChangesProcessed(state) => {
                        BluerEvent::DevicesChanged { state }
                    }
                    BluerSessionEvent::RequestResponse {
                        req,
                        state,
                        err_msg,
                    } => BluerEvent::RequestResponse {
                        req,
                        state,
                        err_msg,
                    },
                    BluerSessionEvent::AgentEvent(e) => BluerEvent::AgentEvent(e),
                };

                let _ = output.send(message).await;
            };

            let mut interval = tokio::time::interval(Duration::from_secs(10));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                let Some(mut session_rx) = session_state.rx.take() else {
                    break;
                };

                if let Some(event) = session_rx.recv().await {
                    event_handler(event).await;
                    // Consume any additional available events
                    let mut count = 0;
                    while let Ok(event) = session_rx.try_recv() {
                        event_handler(event).await;
                        count += 1;
                        if count == 100 {
                            break;
                        }
                    }
                } else {
                    break;
                }

                session_state.rx = Some(session_rx);
                interval.tick().await;
            }

            let _ = output.send(BluerEvent::Finished).await;
            futures::future::pending().await
        })
    }
    )
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn bluer_state(adapter: &Adapter) -> BluerState {
    let (devices, bluetooth_enabled) = futures::join!(
        build_device_list(Vec::new(), adapter),
        adapter.is_powered().map(Result::unwrap_or_default),
    );

    BluerState {
        devices,
        bluetooth_enabled,
    }
}

async fn build_device_list(mut devices: Vec<BluerDevice>, adapter: &Adapter) -> Vec<BluerDevice> {
    let addrs: Vec<Address> = adapter
        .device_addresses()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|addr| !devices.iter().any(|d| d.address == *addr))
        .collect();

    devices.clear();
    if addrs.len() > devices.capacity() {
        devices.reserve(addrs.len() - devices.capacity());
    }

    let mut device_stream = addrs
        .into_iter()
        .filter_map(|address| adapter.device(address).ok())
        .map(async move |device| BluerDevice::from_device(&device).await)
        .collect::<FuturesUnordered<_>>();

    while let Some(device) = device_stream.next().await {
        devices.push(device);
    }

    devices.sort_unstable();
    devices
}
