use crate::{DataLayout, SM64LayoutError};

/// Return the layout for SM64 on the N64, including data added by [DataLayout::add_sm64_extras].
///
/// `version` can be "us", "jp", "eu", or "sh".
///
/// ```
/// for version in ["us", "jp", "eu", "sh"] {
///     wafel_layout::load_sm64_n64_layout(version).unwrap();
/// }
/// ```
pub fn load_sm64_n64_layout(version: &str) -> Result<DataLayout, SM64LayoutError> {
    let bytes = match version.to_lowercase().as_str() {
        "jp" | "j" => &include_bytes!("../n64_layout/sm64_jp.json")[..],
        "us" | "u" => &include_bytes!("../n64_layout/sm64_us.json")[..],
        "eu" | "pal" => &include_bytes!("../n64_layout/sm64_eu.json")[..],
        "sh" => &include_bytes!("../n64_layout/sm64_sh.json")[..],
        _ => return Err(SM64LayoutError::UnknownVersion(version.to_string())),
    };

    let mut layout: DataLayout =
        serde_json::from_slice(bytes).expect("failed to deserialize n64 layout");

    layout.add_sm64_extras()?;

    Ok(layout)
}
