// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod config;
mod i18n;
mod subscriptions;
mod views;

fn main() -> cosmic::iced::Result {
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);
    cosmic::applet::run::<app::AppModel>(())
}
