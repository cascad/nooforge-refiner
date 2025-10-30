use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Length};

use crate::ui::components;
use crate::{Message, NooforgeApp, Tab};

impl NooforgeApp {
    pub fn view_ingest(&self) -> iced::Element<Message> {
        let text_section = column![
            text("Ingest Text").size(24),
            text_input("Enter text to ingest...", &self.ingest_text)
                .on_input(Message::IngestTextChanged)
                .on_submit(Message::IngestTextSubmit)
                .padding(10),
            button(if self.ingest_text_loading {
                "Processing..."
            } else {
                "Submit"
            })
            .on_press_maybe(
                if !self.ingest_text.is_empty() && !self.ingest_text_loading {
                    Some(Message::IngestTextSubmit)
                } else {
                    None
                }
            )
            .padding(10),
            Space::with_height(10),
            components::create_result_view(
                "Result:",
                &self.ingest_text_result,
                self.ingest_text_search_visible,
                &self.ingest_text_search_query,
                Message::ToggleIngestTextSearch,
                Message::IngestTextSearchQueryChanged,
                self.ingest_file_search_visible,
            ),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::FillPortion(1));

        let file_path_display = if let Some(path) = &self.ingest_file_path {
            format!("File: {}", path.display())
        } else {
            "No file selected (drag & drop file here or click button)".to_string()
        };

        let file_section = column![
            text("Ingest File").size(24),
            crate::ui::components::bordered_container(
                column![
                    text(file_path_display.clone()).size(14),
                    Space::with_height(10),
                    text("Drop file anywhere in this window")
                        .size(12)
                        .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                ]
                .align_x(Alignment::Center)
            ),
            button(if self.ingest_file_loading {
                "Processing..."
            } else {
                "Pick File"
            })
            .on_press_maybe(if !self.ingest_file_loading {
                Some(Message::IngestFilePickButton)
            } else {
                None
            })
            .padding(10),
            Space::with_height(10),
            components::create_result_view(
                "Result:",
                &self.ingest_file_result,
                self.ingest_file_search_visible,
                &self.ingest_file_search_query,
                Message::ToggleIngestFileSearch,
                Message::IngestFileSearchQueryChanged,
                self.ingest_text_search_visible,
            ),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::FillPortion(1));

        column![text_section, file_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    pub fn view_rag(&self) -> iced::Element<Message> {
        column![
            text("RAG Query").size(24),
            text_input("Enter query...", &self.rag_query)
                .on_input(Message::RagQueryChanged)
                .on_submit(Message::RagSubmit)
                .padding(10),
            row![
                text("Limit:").size(14),
                text_input("5", &self.rag_limit)
                    .on_input(Message::RagLimitChanged)
                    .width(Length::Fixed(100.0)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            button(if self.rag_loading {
                "Searching..."
            } else {
                "Search"
            })
            .on_press_maybe(if !self.rag_query.is_empty() && !self.rag_loading {
                Some(Message::RagSubmit)
            } else {
                None
            })
            .padding(10),
            Space::with_height(10),
            components::create_result_view(
                "Result:",
                &self.rag_result,
                self.rag_search_visible,
                &self.rag_search_query,
                Message::ToggleRagSearch,
                Message::RagSearchQueryChanged,
                false,
            ),
        ]
        .spacing(10)
        .padding(20)
        .into()
    }

    pub fn view_search(&self) -> iced::Element<Message> {
        column![
            text("Search").size(24),
            text_input("Enter search query...", &self.search_query)
                .on_input(Message::SearchQueryChanged)
                .on_submit(Message::SearchSubmit)
                .padding(10),
            row![
                text("Limit:").size(14),
                text_input("10", &self.search_limit)
                    .on_input(Message::SearchLimitChanged)
                    .width(Length::Fixed(100.0)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            button(if self.search_loading {
                "Searching..."
            } else {
                "Search"
            })
            .on_press_maybe(if !self.search_query.is_empty() && !self.search_loading {
                Some(Message::SearchSubmit)
            } else {
                None
            })
            .padding(10),
            Space::with_height(10),
            components::create_result_view(
                "Result:",
                &self.search_result,
                self.search_search_visible,
                &self.search_search_query,
                Message::ToggleResultSearch,
                Message::ResultSearchQueryChanged,
                false,
            ),
        ]
        .spacing(10)
        .padding(20)
        .into()
    }
}
