@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let st = frag_coord.xy / Resolution;
    return vec4<f32>(st.x, st.y, 0.0, 1.);
}
