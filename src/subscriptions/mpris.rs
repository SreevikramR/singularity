// SPDX-License-Identifier: GPL-3.0-only
//
// MPRIS subscription adapted from cosmic-applet-audio.
// Watches for active media players and emits metadata updates.

use std::{borrow::Cow, fmt::Debug, hash::Hash, path::PathBuf};

use cosmic::iced::{self, Subscription, stream};
use cosmic::iced_futures::futures::{self, SinkExt};
use mpris2_zbus::{
    enumerator,
    media_player::MediaPlayer,
    player::{PlaybackStatus, Player},
};
use tokio::join;
use urlencoding::decode;
use zbus::{
    Connection,
    names::OwnedBusName,
};

#[derive(Clone, Debug)]
pub struct PlayerStatus {
    pub player: Player,
    pub icon: Option<PathBuf>,
    pub title: Option<Cow<'static, str>>,
    pub artists: Option<Vec<Cow<'static, str>>>,
    pub status: PlaybackStatus,
    pub can_pause: bool,
    pub can_play: bool,
    pub can_go_previous: bool,
    pub can_go_next: bool,
}

impl PlayerStatus {
    async fn new(player: Player) -> Option<Self> {
        let metadata = player.metadata().await.ok()?;
        let pathname = metadata.url().unwrap_or_default();
        let pathbuf = PathBuf::from(pathname);

        let title = metadata
            .title()
            .or(pathbuf
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| decode(s).map_or(None, |s| Some(s.into_owned()))))
            .map(Cow::from);
        let artists = metadata
            .artists()
            .map(|a| a.into_iter().map(Cow::from).collect::<Vec<_>>());
        let icon = metadata
            .art_url()
            .and_then(|u| url::Url::parse(&u).ok())
            .and_then(|u| {
                if u.scheme() == "file" {
                    u.to_file_path().ok()
                } else {
                    None
                }
            });

        let (playback_status, can_pause, can_play, can_go_previous, can_go_next) = join!(
            player.playback_status(),
            player.can_pause(),
            player.can_play(),
            player.can_go_previous(),
            player.can_go_next()
        );
        Some(Self {
            icon,
            title,
            artists,
            status: playback_status.unwrap_or(PlaybackStatus::Stopped),
            can_pause: can_pause.unwrap_or_default(),
            can_play: can_play.unwrap_or_default(),
            can_go_previous: can_go_previous.unwrap_or_default(),
            can_go_next: can_go_next.unwrap_or_default(),
            player,
        })
    }
}

#[derive(Clone, Debug)]
pub enum MprisUpdate {
    Setup,
    Player(PlayerStatus),
    Finished,
}

#[derive(Clone, Debug)]
pub enum MprisRequest {
    Play,
    Pause,
    Next,
    Previous,
}

struct MprisPlayer {
    player: Player,
    #[allow(dead_code)]
    media_player: MediaPlayer,
}

impl MprisPlayer {
    async fn new(conn: &Connection, name: OwnedBusName) -> mpris2_zbus::error::Result<Self> {
        Ok(Self {
            player: Player::new(conn, name.clone()).await?,
            media_player: MediaPlayer::new(conn, name).await?,
        })
    }

    #[allow(dead_code)]
    fn name(&self) -> &zbus::names::BusName<'_> {
        self.player.inner().destination()
    }
}

pub fn mpris_subscription<I: 'static + Hash + Copy + Send + Sync + Debug>(
    id: I,
) -> iced::Subscription<MprisUpdate> {
    Subscription::run_with(id, |_id: &I| {
        stream::channel(50, move |mut output| async move {
            run(&mut output).await;
            let _ = output.send(MprisUpdate::Finished).await;
            futures::future::pending().await
        })
    })
}

async fn run(output: &mut futures::channel::mpsc::Sender<MprisUpdate>) {
    let Ok(conn) = Connection::session().await else {
        return;
    };

    let Ok(enumerator) = enumerator::Enumerator::new(&conn).await else {
        return;
    };

    let Ok(names) = enumerator.players().await else {
        return;
    };

    // Connect to the first available player
    let mut active_player: Option<MprisPlayer> = None;
    for name in names {
        if let Ok(mp) = MprisPlayer::new(&conn, name).await {
            if let Some(status) = PlayerStatus::new(mp.player.clone()).await {
                let _ = output.send(MprisUpdate::Player(status)).await;
                active_player = Some(mp);
                break;
            }
        }
    }

    // If no player found, just send setup and wait
    if active_player.is_none() {
        let _ = output.send(MprisUpdate::Setup).await;
    }

    // Keep polling for updates (simple polling approach)
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        if let Some(ref mp) = active_player {
            if let Some(status) = PlayerStatus::new(mp.player.clone()).await {
                let _ = output.send(MprisUpdate::Player(status)).await;
            }
        } else {
            // Try to find a new player
            if let Ok(names) = enumerator.players().await {
                for name in names {
                    if let Ok(mp) = MprisPlayer::new(&conn, name).await {
                        if let Some(status) = PlayerStatus::new(mp.player.clone()).await {
                            let _ = output.send(MprisUpdate::Player(status)).await;
                            active_player = Some(mp);
                            break;
                        }
                    }
                }
            }
        }
    }
}
