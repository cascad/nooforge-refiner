use iced::{Alignment, Length};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};

use crate::Message; // ← конкретный тип из main.rs

pub fn create_result_view<'a>(
    title: &'a str,
    result_text: &'a str,
    search_visible: bool,
    search_query: &'a str,
    toggle_search_msg: Message,
    search_query_changed_msg: fn(String) -> Message,
    dimmed: bool,
) -> iced::Element<'a, Message> {
    let mut col = column![].spacing(5);

    let header = row![
        text(title).size(16),
        Space::with_width(Length::Fill),
        button("Find (Ctrl+F)")
            .on_press(toggle_search_msg)
            .padding(5),
    ]
    .align_y(Alignment::Center);

    col = col.push(header);

    if search_visible {
        let search_bar = row![
            text("Find:").size(14),
            text_input("Search in results...", search_query)
                .on_input(search_query_changed_msg)
                .padding(5)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        col = col.push(search_bar);
    }

    let display_text = if result_text.is_empty() {
        "No results yet".to_string()
    } else {
        result_text.to_string()
    };

    let content = container(text(display_text).size(13))
        .padding(10)
        .width(Length::Fill);

    let scrollable_content = scrollable(content).height(Length::Fill);

    let final_content: iced::Element<'a, Message> = if dimmed {
        container(scrollable_content)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.3))),
                ..container::Style::default()
            })
            .into()
    } else {
        scrollable_content.into()
    };

    col.push(final_content).into()
}

// ✅ Убираем обобщение — используем конкретный crate::Message
pub fn bordered_container<'a>(
    content: impl Into<iced::Element<'a, Message>>,
) -> iced::Element<'a, Message> {
    container(content)
        .padding(20)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: palette.primary.weak.color,
                },
                ..container::Style::default()
            }
        })
        .into()
}