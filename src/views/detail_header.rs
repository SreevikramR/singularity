// SPDX-License-Identifier: GPL-3.0-only
//
// Shared header component for all detail sub-menu views.
// Renders: [← Back]  [Title]  [On/Off Toggle]

use crate::app::Message;
use cosmic::iced::widget::row;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, text};

/// Renders the detail view header bar.
///
/// - `title`: The title text (e.g., "Wi-Fi Networks")
/// - `enabled`: Current on/off state for the master toggle
/// - `on_back`: Message to send when back button is clicked
/// - `on_toggle`: Message to send when the toggle is flipped
pub fn detail_header<'a>(
    title: &str,
    enabled: bool,
    on_back: Message,
    on_toggle: Message,
) -> Element<'a, Message> {
    let back_btn = button::icon(icon::from_name("go-previous-symbolic").size(20).symbolic(true))
        .on_press(on_back)
        .padding(4);

    let title_text = text::heading(title.to_string())
        .width(Length::Fill)
        .align_x(Alignment::Center);

    let toggle_icon = if enabled {
        "emblem-enabled-symbolic"
    } else {
        "window-close-symbolic"
    };

    let toggle_btn = button::icon(icon::from_name(toggle_icon).size(20).symbolic(true))
        .on_press(on_toggle)
        .padding(4);

    row![back_btn, title_text, toggle_btn]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
}
