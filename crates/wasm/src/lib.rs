use wasm_bindgen::prelude::*;

use qp_analyzer::get_override_labels;

#[wasm_bindgen]
pub fn override_labels(schema_str: &str) -> Result<Vec<String>, String> {
    let override_labels = get_override_labels(schema_str).map_err(|e| e.to_string())?;
    Ok(override_labels.iter().map(|s| s.to_string()).collect())
}
