fn plot(st: vec2<f32>) -> f32 {
    return smoothstep(0.02, 0.0, abs(st.y - st.x));
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {

	let st = frag_coord.xy/Resolution;

    let y = st.x;

    var color = vec3<f32>(y);

    let pct = plot(st);
    color = (1.0-pct)*color+pct*vec3<f32>(0.0,1.0,0.0);

	return vec4<f32>(color,1.0);
}

