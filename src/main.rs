// SPDX-License-Identifier: GPL-3.0-only

pub mod app;
pub mod bluetooth;
pub mod config;
pub mod network;
mod i18n;
mod subscriptions;
mod views;

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt::init();
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);
    cosmic::applet::run::<app::AppModel>(())
}
