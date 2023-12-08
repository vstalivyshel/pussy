use crate::bind::ShaderBindings;
use crate::ctx::{FS_ENTRY, VS_ENTRY};
use naga::{front::wgsl, valid};
use std::path::Path;

pub struct ShaderSource(String);

impl Default for ShaderSource {
    fn default() -> Self {
        Self(format!(
            r#" {vertex_main}

        @fragment
        fn {FS_ENTRY}() -> @location(0) vec4<f32> {{
            return vec4<f32>(0.1, 0.2, 0.3, 1.);
        }}
        "#,
            vertex_main = generate_vertex_main(),
        ))
    }
}

impl ShaderSource {
    pub fn validate(path: impl AsRef<Path>, bindings: &ShaderBindings) -> Result<Self, String> {
        // TODO: catch redefenition of function
        let path = path.as_ref();
        let loaded = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        let mut source = loaded + &bindings.as_wgsl_string();
        let module = wgsl::parse_str(&source)
            .map_err(|e| e.emit_to_string_with_path(&source, path.to_str().unwrap()))?;

        valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
            .validate(&module)
            .map_err(|e| e.emit_to_string_with_path(&source, path.to_str().unwrap()))?;

        let entries = module.entry_points;

        if !entries.iter().any(|ep| ep.name.contains(VS_ENTRY)) {
            source += &generate_vertex_main();
        }

        if !entries.iter().any(|ep| ep.name.contains(FS_ENTRY)) {
            return Err(format!(
                "{path:?} parsing error: `{FS_ENTRY}` entrie not found in source"
            ));
        }

        Ok(Self(source))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn generate_vertex_main() -> String {
    format!(
r#"
@vertex
fn {VS_ENTRY}(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {{
    var x = f32(i32((vertex_index << u32(1)) & u32(2)));
    var y = f32(i32(vertex_index & u32(2)));
    var uv = vec2<f32>(x, y);
    var out = 2.0 * uv - vec2<f32>(1.0, 1.0);
    return vec4<f32>(out.x, out.y, 0.0, 1.0);
}}"#
    )
}
