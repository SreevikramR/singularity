use cosmic_settings_network_manager_subscription::{
    available_wifi::AccessPoint,
    hw_address::HwAddress,
    nm_secret_agent::{PasswordFlag, SecretSender},
};
use secure_string::SecureString;

#[derive(Debug, Clone)]
pub enum NewConnectionState {
    EnterPassword {
        access_point: AccessPoint,
        description: Option<String>,
        identity: String,
        password: SecureString,
        password_hidden: bool,
    },
    Waiting(AccessPoint),
    Failure(AccessPoint),
}

impl NewConnectionState {
    pub fn ssid(&self) -> &str {
        match self {
            Self::EnterPassword { access_point, .. } => &access_point.ssid,
            Self::Waiting(ap) => &ap.ssid,
            Self::Failure(ap) => &ap.ssid,
        }
    }
    pub fn hw_address(&self) -> HwAddress {
        match self {
            Self::EnterPassword { access_point, .. } => access_point.hw_address,
            Self::Waiting(ap) => ap.hw_address,
            Self::Failure(ap) => ap.hw_address,
        }
    }
}

impl From<NewConnectionState> for AccessPoint {
    fn from(connection_state: NewConnectionState) -> Self {
        match connection_state {
            NewConnectionState::EnterPassword { access_point, .. } => access_point,
            NewConnectionState::Waiting(access_point) => access_point,
            NewConnectionState::Failure(access_point) => access_point,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestedVpn {
    pub name: String,
    pub uuid: std::sync::Arc<str>,
    pub description: Option<String>,
    pub password: SecureString,
    pub password_hidden: bool,
    pub tx: SecretSender,
}

#[derive(Clone, Debug)]
pub enum ConnectionSettings {
    Vpn(VpnConnectionSettings),
    Wireguard { id: String },
}

#[derive(Clone, Debug, Default)]
pub struct VpnConnectionSettings {
    pub id: String,
    pub username: Option<String>,
    pub connection_type: Option<ConnectionType>,
    pub password_flag: Option<PasswordFlag>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionType {
    Password,
}

impl VpnConnectionSettings {
    pub fn password_flag(&self) -> Option<PasswordFlag> {
        self.connection_type
            .as_ref()
            .is_some_and(|ct| match ct {
                ConnectionType::Password => true,
            })
            .then_some(self.password_flag)
            .flatten()
    }
}

#[derive(Debug, Clone)]
pub struct Password {
    pub ssid: cosmic_settings_network_manager_subscription::SSID,
    pub hw_address: HwAddress,
    pub identity: Option<String>,
    pub password: SecureString,
    pub password_hidden: bool,
    pub tx: SecretSender,
}


use anyhow::Context;
use cosmic::Task;
use std::sync::Arc;
use zbus::zvariant::ObjectPath;
use cosmic_dbus_networkmanager::settings::{NetworkManagerSettings, connection::Settings};
use cosmic_settings_network_manager_subscription::{self as network_manager, NetworkManagerState};
use futures::StreamExt;
use cosmic::Apply;
use indexmap::IndexMap;


pub fn update_state(conn: zbus::Connection) -> Task<crate::app::Message> {
    cosmic::task::future(async move {
        match NetworkManagerState::new(&conn).await {
            Ok(state) => crate::app::Message::UpdateState(state),
            Err(why) => crate::app::Message::Error(why.to_string()),
        }
    })
}

pub fn update_devices(conn: zbus::Connection) -> Task<crate::app::Message> {
    cosmic::task::future(async move {
        let filter =
            |device_type| matches!(device_type, cosmic_dbus_networkmanager::interface::enums::DeviceType::Wifi);
        match cosmic_settings_network_manager_subscription::devices::list(&conn, filter).await {
            Ok(devices) => crate::app::Message::UpdateDevices(devices),
            Err(why) => crate::app::Message::Error(why.to_string()),
        }
    })
}



pub fn load_vpns(conn: zbus::Connection) -> Task<crate::app::Message> {
    let settings = async move {
        let settings = network_manager::dbus::settings::NetworkManagerSettings::new(&conn).await?;

        _ = settings.load_connections(&[]).await;

        let settings = settings
            // Get a list of known connections.
            .list_connections()
            .await?
            // Prepare for wrapping in a concurrent stream.
            .into_iter()
            .map(|conn| async move { conn })
            // Create a concurrent stream for each connection.
            .apply(futures::stream::FuturesOrdered::from_iter)
            // Concurrently fetch settings for each connection, and filter for VPN.
            .filter_map(|conn| async move {
                let settings = conn.get_settings().await.ok()?;

                let connection = settings.get("connection")?;

                match connection
                    .get("type")?
                    .downcast_ref::<String>()
                    .ok()?
                    .as_str()
                {
                    "vpn" => (),

                    "wireguard" => {
                        let id = connection.get("id")?.downcast_ref::<String>().ok()?;
                        let uuid = connection.get("uuid")?.downcast_ref::<String>().ok()?;
                        return Some((Arc::from(uuid), ConnectionSettings::Wireguard { id }));
                    }

                    _ => return None,
                }

                let vpn = settings.get("vpn")?;
                let id = connection.get("id")?.downcast_ref::<String>().ok()?;
                let uuid = connection.get("uuid")?.downcast_ref::<String>().ok()?;

                let (connection_type, username, password_flag) = vpn
                    .get("data")
                    .and_then(|data| data.downcast_ref::<zbus::zvariant::Dict>().ok())
                    .map(|dict| {
                        let (mut connection_type, mut password_flag) = (None, None);
                        let mut username = vpn
                            .get("user-name")
                            .and_then(|u| u.downcast_ref::<String>().ok());
                        if dict
                            .get::<String, String>(&String::from("connection-type"))
                            .ok()
                            .flatten()
                            .as_deref()
                            // may be "password" or "password-tls"
                            .is_some_and(|p| p.starts_with("password"))
                        {
                            connection_type = Some(ConnectionType::Password);
                            username = Some(username.unwrap_or_default());

                            password_flag = dict
                                .get::<String, String>(&String::from("password-flags"))
                                .ok()
                                .flatten()
                                .and_then(|value| match value.as_str() {
                                    "0" => Some(PasswordFlag::None),
                                    "1" => Some(PasswordFlag::AgentOwned),
                                    "2" => Some(PasswordFlag::NotSaved),
                                    "4" => Some(PasswordFlag::NotRequired),
                                    _ => None,
                                });
                        }

                        (connection_type, username, password_flag)
                    })
                    .unwrap_or_default();

                Some((
                    Arc::from(uuid),
                    ConnectionSettings::Vpn(VpnConnectionSettings {
                        id,
                        connection_type,
                        password_flag,
                        username,
                    }),
                ))
            })
            // Reduce the settings list into
            .fold(IndexMap::new(), |mut set, (uuid, data)| async move {
                set.insert(uuid, data);
                set
            })
            .await;

        Ok::<_, zbus::Error>(settings)
    };

    cosmic::task::future(async move {
        settings.await.map_or_else(
            |why| crate::app::Message::Error(why.to_string()),
            crate::app::Message::KnownConnections,
        )
    })
}

pub fn connect_vpn(
    conn: zbus::Connection,
    tx: futures::channel::mpsc::UnboundedSender<network_manager::Request>,
    uuid: Arc<str>,
) -> Task<crate::app::Message> {
    cosmic::task::future(async move {
        // Find the connection by UUID
        if let Ok(nm_settings) = NetworkManagerSettings::new(&conn).await {
            if let Ok(connections) = nm_settings.list_connections().await {
                for connection in connections {
                    if let Ok(settings) = connection.get_settings().await {
                        let settings = Settings::new(settings);
                        if let Some(conn_settings) = &settings.connection {
                            if conn_settings.uuid.as_ref().is_some_and(|conn_uuid| {
                                conn_uuid.as_str() == uuid.as_ref()
                            }) {
                                let path = connection.inner().path().clone().to_owned();
                                if let Err(err) =
                                    tx.unbounded_send(network_manager::Request::Activate(
                                        ObjectPath::try_from("/").unwrap(),
                                        path,
                                    ))
                                {
                                    if err.is_disconnected() {
                                        return zbus::Connection::system()
                                            .await
                                            .context("failed to create system dbus connection")
                                            .map_or_else(
                                                |why| crate::app::Message::Error(why.to_string()),
                                                crate::app::Message::NetworkManagerConnect,
                                            );
                                    }

                                    tracing::error!("{err:?}");
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        crate::app::Message::Refresh
    })
}

pub fn system_conn() -> Task<crate::app::Message> {
    cosmic::Task::future(async move {
        zbus::Connection::system()
            .await
            .context("failed to create system dbus connection")
            .map_or_else(
                |why| crate::app::Message::Error(why.to_string()),
                crate::app::Message::NetworkManagerConnect,
            )
    })
}
