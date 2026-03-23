struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) uv: vec2<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) packed_params: vec2<u32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) params_a: vec4<f32>,
  @location(3) params_b: vec4<f32>,
};

fn unpack_u8x4(word: u32) -> vec4<f32> {
  return vec4<f32>(
    f32(word & 0xFFu) / 255.0,
    f32((word >> 8u) & 0xFFu) / 255.0,
    f32((word >> 16u) & 0xFFu) / 255.0,
    f32((word >> 24u) & 0xFFu) / 255.0,
  );
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  output.clip_position = vec4<f32>(input.position, 1.0);
  output.uv = input.uv;
  output.normal = normalize(input.normal);
  output.params_a = unpack_u8x4(input.packed_params.x);
  output.params_b = unpack_u8x4(input.packed_params.y);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let alpha_power = mix(1.2, 3.4, input.params_a.x);
  let edge_softness = mix(0.08, 0.45, input.params_a.y);
  let normal_bend = mix(0.2, 1.1, input.params_a.z);
  let backlight_boost = mix(0.1, 0.85, input.params_a.w);
  let hue_shift = input.params_b.x * 2.0 - 1.0;
  let canopy_height_t = input.params_b.y;

  let centered_uv = input.uv * 2.0 - vec2<f32>(1.0, 1.0);
  let radial = clamp(1.0 - dot(centered_uv, centered_uv), 0.0, 1.0);
  let vertical = pow(clamp(input.uv.y, 0.0, 1.0), alpha_power);
  let edge = smoothstep(0.0, edge_softness + 0.02, radial);
  let alpha = edge * vertical;
  if (alpha < 0.08) {
    discard;
  }

  let view_light = normalize(vec3<f32>(-0.35, 0.55, 0.76));
  let bent_normal = normalize(input.normal + vec3<f32>(centered_uv.x, centered_uv.y * 0.45, 0.0) * normal_bend);
  let ndotl = clamp(dot(bent_normal, view_light), 0.0, 1.0);
  let rim = pow(1.0 - clamp(abs(centered_uv.x), 0.0, 1.0), 2.0);

  let base = vec3<f32>(
    0.09 + canopy_height_t * 0.04 + hue_shift * 0.03,
    0.21 + canopy_height_t * 0.23,
    0.08 + canopy_height_t * 0.05 - hue_shift * 0.02,
  );
  let lit = base * (0.55 + ndotl * (0.55 + backlight_boost)) + vec3<f32>(0.15, 0.17, 0.06) * rim * backlight_boost;

  return vec4<f32>(lit, alpha);
}
