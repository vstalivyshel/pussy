use crate::bind::ShaderBindings;
use crate::ctx::{FS_ENTRY, VS_ENTRY};
use naga::{front::wgsl, valid};
use std::path::Path;

pub struct ShaderSource(String);

impl Default for ShaderSource {
    fn default() -> Self {
        Self(format!(
            r#"
        @vertex
        fn {VS_ENTRY}() -> @builtin(position) vec4<f32> {{
            return vec4<f32>(0., 0., 0., 0.);
        }}

        @fragment
        fn {FS_ENTRY}() -> @location(0) vec4<f32> {{
            return vec4<f32>(0.1, 0.2, 0.3, 1.);
        }}
        "#
        ))
    }
}

impl ShaderSource {
    pub fn validate(path: impl AsRef<Path>, bindings: &ShaderBindings) -> Result<Self, String> {
        let path = path.as_ref();
        let loaded = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        let source = bindings.as_wgsl_string() + &loaded;
        let module = wgsl::parse_str(&source)
            .map_err(|e| format!("{path:?} parsing {err}", err = e.emit_to_string(&source)))?;

        valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
            .validate(&module)
            .map_err(|e| format!("{path:?} parsing {err}", err = e.emit_to_string(&source)))?;

        let mut entries = module.entry_points.iter();

        // Naga's validator doesn't know about entries
        if !entries.any(|ep| ep.name.contains(VS_ENTRY)) {
            return Err(format!(
                "{path:?} parsing error: `{VS_ENTRY}` entrie not found in source"
            ));
        } else if !entries.any(|ep| ep.name.contains(FS_ENTRY)) {
            return Err(format!(
                "{path:?} parsing error: `{FS_ENTRY}` entrie not found in source"
            ));
        }

        Ok(Self(source))
    }

    pub fn _into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
