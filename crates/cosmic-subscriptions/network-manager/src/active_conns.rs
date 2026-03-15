// Copyright 2024 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::Event;
use cosmic_dbus_networkmanager::nm::NetworkManager;
use futures::{SinkExt, StreamExt};
use iced_futures::{Subscription, stream};
use zbus::Connection;

#[derive(Debug, Clone)]
pub enum State {
    Continue(Connection),
    Error,
}

pub fn active_conns_subscription() -> iced_futures::Subscription<Event> {
    Subscription::run_with(
        "singularity-active-conns",
        |_: &&str| stream::channel(50, |output| async move {
            let conn = zbus::Connection::system().await.unwrap();
            watch(conn, output).await;
            futures::future::pending().await
        }),
    )
}

pub async fn watch(conn: zbus::Connection, mut output: futures::channel::mpsc::Sender<Event>) {
    let mut state = State::Continue(conn);

    loop {
        state = start_listening(state, &mut output).await;
    }
}

async fn start_listening(
    state: State,
    output: &mut futures::channel::mpsc::Sender<Event>,
) -> State {
    let conn = match state {
        State::Continue(conn) => conn,
        State::Error => futures::future::pending().await,
    };
    let network_manager = match NetworkManager::new(&conn).await {
        Ok(n) => n,
        Err(why) => {
            tracing::error!(why = why.to_string(), "Failed to connect to NetworkManager");
            return State::Error;
        }
    };

    let mut active_conns_changed = network_manager.receive_active_connections_changed().await;
    active_conns_changed.next().await;

    while let (Some(_change), _) = futures::future::join(
        active_conns_changed.next(),
        tokio::time::sleep(tokio::time::Duration::from_secs(1)),
    )
    .await
    {
        _ = output.send(Event::ActiveConns).await;
    }

    State::Continue(conn)
}
