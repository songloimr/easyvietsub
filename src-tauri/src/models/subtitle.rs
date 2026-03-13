use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
    pub translated_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderStyle {
    pub font_family: String,
    pub font_size: u64,
    pub text_color: String,
    pub outline_color: String,
    pub outline_width: u64,
    pub background_color: String,
    pub line_spacing: u64,
    pub margin_x: u64,
    pub margin_y: u64,
    pub bold: bool,
    pub italic: bool,
    pub position: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    pub input_path: String,
    pub output_path: String,
    pub subtitle_content: String,
    pub style: RenderStyle,
}
