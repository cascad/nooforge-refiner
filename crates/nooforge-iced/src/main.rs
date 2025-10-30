mod api;
mod app;
mod ui;
mod utils;

use iced::Size;

#[derive(Debug, Clone)]
pub enum Tab {
    Ingest,
    Rag,
    Search,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(Tab),
    IngestTextChanged(String),
    IngestTextSubmit,
    IngestTextResult(Result<String, String>),
    IngestFilePickButton,
    IngestFileSelected(Option<std::path::PathBuf>),
    IngestFileDropped(String),
    IngestFileSubmit(std::path::PathBuf),
    IngestFileResult(Result<String, String>),
    FileHovered(std::path::PathBuf),
    FilesHoveredLeft,
    RagQueryChanged(String),
    RagLimitChanged(String),
    RagSubmit,
    RagResult(Result<String, String>),
    SearchQueryChanged(String),
    SearchLimitChanged(String),
    SearchSubmit,
    SearchResult(Result<String, String>),
    ToggleIngestTextSearch,
    ToggleIngestFileSearch,
    ToggleRagSearch,
    ToggleResultSearch,
    IngestTextSearchQueryChanged(String),
    IngestFileSearchQueryChanged(String),
    RagSearchQueryChanged(String),
    ResultSearchQueryChanged(String),
    EventOccurred(iced::Event),
}
pub struct NooforgeApp {
    pub current_tab: Tab,
    pub ingest_text: String,
    pub ingest_text_result: String,
    pub ingest_text_loading: bool,
    pub ingest_text_search_visible: bool,
    pub ingest_text_search_query: String,
    pub ingest_file_path: Option<std::path::PathBuf>,
    pub ingest_file_result: String,
    pub ingest_file_loading: bool,
    pub ingest_file_search_visible: bool,
    pub ingest_file_search_query: String,
    pub rag_query: String,
    pub rag_limit: String,
    pub rag_result: String,
    pub rag_loading: bool,
    pub rag_search_visible: bool,
    pub rag_search_query: String,
    pub search_query: String,
    pub search_limit: String,
    pub search_result: String,
    pub search_loading: bool,
    pub search_search_visible: bool,
    pub search_search_query: String,
}

impl Default for NooforgeApp {
    fn default() -> Self {
        Self {
            current_tab: Tab::Ingest,
            ingest_text: String::new(),
            ingest_text_result: String::new(),
            ingest_text_loading: false,
            ingest_text_search_visible: false,
            ingest_text_search_query: String::new(),
            ingest_file_path: None,
            ingest_file_result: String::new(),
            ingest_file_loading: false,
            ingest_file_search_visible: false,
            ingest_file_search_query: String::new(),
            rag_query: String::new(),
            rag_limit: String::from("10"),
            rag_result: String::new(),
            rag_loading: false,
            rag_search_visible: false,
            rag_search_query: String::new(),
            search_query: String::new(),
            search_limit: String::from("10"),
            search_result: String::new(),
            search_loading: false,
            search_search_visible: false,
            search_search_query: String::new(),
        }
    }
}

pub fn main() -> iced::Result {
    iced::application("Nooforge", app::update, app::view)
        .subscription(app::subscription)
        .theme(app::theme)
        .window_size(Size::new(1200.0, 900.0))
        .run()
}
