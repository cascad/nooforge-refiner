use iced::event;
use iced::window;
use iced::{Event, Subscription, Task};
use iced::keyboard;
use crate::Tab;
use crate::{Message, NooforgeApp};

pub fn update(state: &mut NooforgeApp, message: Message) -> Task<Message> {
    match message {
        Message::TabSelected(tab) => {
            state.current_tab = tab;
            Task::none()
        }
        Message::IngestTextChanged(text) => {
            state.ingest_text = text;
            Task::none()
        }
        Message::IngestTextSubmit => {
            if state.ingest_text.is_empty() || state.ingest_text_loading {
                return Task::none();
            }
            state.ingest_text_loading = true;
            state.ingest_text_result = "Loading...".to_string();
            let text = state.ingest_text.clone();
            Task::perform(
                async move { crate::api::ingest_text_api(text).await },
                Message::IngestTextResult,
            )
        }
        Message::IngestTextResult(result) => {
            state.ingest_text_loading = false;
            state.ingest_text_result = result.unwrap_or_else(|e| format!("Error: {}", e));
            Task::none()
        }
        Message::IngestFilePickButton => Task::perform(
            async { crate::api::pick_file().await },
            Message::IngestFileSelected,
        ),
        Message::IngestFileSelected(path) => {
            if let Some(p) = path {
                state.ingest_file_path = Some(p.clone());
                update(state, Message::IngestFileSubmit(p))
            } else {
                Task::none()
            }
        }
        Message::IngestFileDropped(path_str) => {
            let clean_path = if path_str.starts_with("file://") {
                path_str.trim_start_matches("file://").to_string()
            } else {
                path_str
            };
            let path = std::path::PathBuf::from(clean_path);
            if !path.exists() {
                state.ingest_file_result = format!("Error: File not found: {}", path.display());
                return Task::none();
            }
            if !path.is_file() {
                state.ingest_file_result = format!("Error: Not a file: {}", path.display());
                return Task::none();
            }
            state.ingest_file_path = Some(path.clone());
            update(state, Message::IngestFileSubmit(path))
        }
        Message::FileHovered(_) | Message::FilesHoveredLeft => Task::none(),
        Message::IngestFileSubmit(path) => {
            if state.ingest_file_loading {
                return Task::none();
            }
            state.ingest_file_loading = true;
            state.ingest_file_result = "Loading...".to_string();
            Task::perform(
                async move { crate::api::ingest_file_api(path).await },
                Message::IngestFileResult,
            )
        }
        Message::IngestFileResult(result) => {
            state.ingest_file_loading = false;
            state.ingest_file_result = result.unwrap_or_else(|e| format!("Error: {}", e));
            Task::none()
        }
        Message::RagQueryChanged(query) => {
            state.rag_query = query;
            Task::none()
        }
        Message::RagLimitChanged(limit) => {
            state.rag_limit = limit;
            Task::none()
        }
        Message::RagSubmit => {
            if state.rag_query.is_empty() || state.rag_loading {
                return Task::none();
            }
            state.rag_loading = true;
            state.rag_result = "Loading...".to_string();
            let query = state.rag_query.clone();
            let limit = state.rag_limit.parse().unwrap_or(5);
            Task::perform(
                async move { crate::api::rag_api(query, limit).await },
                Message::RagResult,
            )
        }
        Message::RagResult(result) => {
            state.rag_loading = false;
            state.rag_result = result.unwrap_or_else(|e| format!("Error: {}", e));
            Task::none()
        }
        Message::SearchQueryChanged(query) => {
            state.search_query = query;
            Task::none()
        }
        Message::SearchLimitChanged(limit) => {
            state.search_limit = limit;
            Task::none()
        }
        Message::SearchSubmit => {
            if state.search_query.is_empty() || state.search_loading {
                return Task::none();
            }
            state.search_loading = true;
            state.search_result = "Loading...".to_string();
            let query = state.search_query.clone();
            let limit = state.search_limit.parse().unwrap_or(10);
            Task::perform(
                async move { crate::api::search_api(query, limit).await },
                Message::SearchResult,
            )
        }
        Message::SearchResult(result) => {
            state.search_loading = false;
            state.search_result = result.unwrap_or_else(|e| format!("Error: {}", e));
            Task::none()
        }
        Message::ToggleIngestTextSearch => {
            state.ingest_text_search_visible = !state.ingest_text_search_visible;
            if !state.ingest_text_search_visible {
                state.ingest_text_search_query.clear();
            }
            Task::none()
        }
        Message::ToggleIngestFileSearch => {
            state.ingest_file_search_visible = !state.ingest_file_search_visible;
            if !state.ingest_file_search_visible {
                state.ingest_file_search_query.clear();
            }
            Task::none()
        }
        Message::ToggleRagSearch => {
            state.rag_search_visible = !state.rag_search_visible;
            if !state.rag_search_visible {
                state.rag_search_query.clear();
            }
            Task::none()
        }
        Message::ToggleResultSearch => {
            state.search_search_visible = !state.search_search_visible;
            if !state.search_search_visible {
                state.search_search_query.clear();
            }
            Task::none()
        }
        Message::IngestTextSearchQueryChanged(query) => {
            state.ingest_text_search_query = query;
            Task::none()
        }
        Message::IngestFileSearchQueryChanged(query) => {
            state.ingest_file_search_query = query;
            Task::none()
        }
        Message::RagSearchQueryChanged(query) => {
            state.rag_search_query = query;
            Task::none()
        }
        Message::ResultSearchQueryChanged(query) => {
            state.search_search_query = query;
            Task::none()
        }
        Message::EventOccurred(event) => {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                let is_f_key = match key {
                    keyboard::Key::Character(ref c) => {
                        let ch = c.to_lowercase();
                        ch == "f" || ch == "а"
                    }
                    _ => false,
                };
                if is_f_key && (modifiers.command() || modifiers.control()) {
                    return match state.current_tab {
                        Tab::Ingest => {
                            if !state.ingest_text_result.is_empty()
                                && !state.ingest_file_result.is_empty()
                            {
                                if !state.ingest_text_search_visible
                                    && !state.ingest_file_search_visible
                                {
                                    update(state, Message::ToggleIngestTextSearch)
                                } else if state.ingest_text_search_visible {
                                    state.ingest_text_search_visible = false;
                                    state.ingest_text_search_query.clear();
                                    update(state, Message::ToggleIngestFileSearch)
                                } else {
                                    state.ingest_file_search_visible = false;
                                    state.ingest_file_search_query.clear();
                                    update(state, Message::ToggleIngestTextSearch)
                                }
                            } else if !state.ingest_text_result.is_empty() {
                                update(state, Message::ToggleIngestTextSearch)
                            } else if !state.ingest_file_result.is_empty() {
                                update(state, Message::ToggleIngestFileSearch)
                            } else {
                                Task::none()
                            }
                        }
                        Tab::Rag => update(state, Message::ToggleRagSearch),
                        Tab::Search => update(state, Message::ToggleResultSearch),
                    };
                }
            }
            Task::none()
        }
    }
}

pub fn view(state: &NooforgeApp) -> iced::Element<Message> {
    let tabs = iced::widget::row![
        iced::widget::button("Ingest")
            .on_press(Message::TabSelected(Tab::Ingest))
            .padding(10),
        iced::widget::button("RAG")
            .on_press(Message::TabSelected(Tab::Rag))
            .padding(10),
        iced::widget::button("Search")
            .on_press(Message::TabSelected(Tab::Search))
            .padding(10),
    ]
    .spacing(10);

    let content = match state.current_tab {
        Tab::Ingest => state.view_ingest(),
        Tab::Rag => state.view_rag(),
        Tab::Search => state.view_search(),
    };

    iced::widget::column![tabs, content]
        .spacing(20)
        .padding(20)
        .into()
}

pub fn subscription(_state: &NooforgeApp) -> Subscription<Message> {
    event::listen().map(|event| match event {
        Event::Window(window::Event::FileHovered(path)) => Message::FileHovered(path),
        Event::Window(window::Event::FileDropped(path)) => {
            Message::IngestFileDropped(path.display().to_string())
        }
        Event::Window(window::Event::FilesHoveredLeft) => Message::FilesHoveredLeft,
        _ => Message::EventOccurred(event),
    })
}

pub fn theme(_state: &NooforgeApp) -> iced::Theme {
    iced::Theme::Dark
}
