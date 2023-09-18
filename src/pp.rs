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
        let source = crate::pp::prelude(bindings) + &loaded;
        let module = wgsl::parse_str(&source)
            .map_err(|e| format!("{path:?} parsing {err}", err = e.emit_to_string(&source)))?;

        valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
            .validate(&module)
            .map_err(|e| format!("{path:?} parsing {err}", err = e.emit_to_string(&source)))?;

        let mut entries = module.entry_points.iter();

        if !entries.any(|ep| ep.name.contains(VS_ENTRY)) {
            return Err(format!(
                "{path:?} parsing error: `{VS_ENTRY}` not found in source"
            ));
        } else if !entries.any(|ep| ep.name.contains(FS_ENTRY)) {
            return Err(format!(
                "{path:?} parsing error: `{FS_ENTRY}` not found in source"
            ));
        }

        Ok(Self(source))
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn prelude(bindings: &ShaderBindings) -> String {
    let mut pre = String::new();
    pre.push_str(&bindings.as_wgsl_string());

    pre
}