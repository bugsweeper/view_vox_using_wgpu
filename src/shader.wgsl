// Vertex shader
struct CameraUniform {
    view_projection: mat4x4<f32>,
    view_position: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct InstanceInput {
    @location(2) position: vec4<u32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> light: Light;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // We don't need (or want) much ambient light, so 0.1 is fine
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;
    
    let world_position = model.position + vec4<f32>(instance.position);
    
    out.clip_position = camera.view_projection * world_position;
    out.color = instance.color;
    out.world_normal = model.normal.xyz;
    out.world_position = world_position.xyz;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // We don't need (or want) much ambient light, so 0.1 is fine
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    let light_direction = normalize(light.position - in.world_position);

    let diffuse_strength = max(dot(in.world_normal, light_direction), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let view_direction = normalize(camera.view_position.xyz - in.world_position);
    let halfway_direction = normalize(view_direction + light_direction);

    let specular_strength = pow(max(dot(in.world_normal, halfway_direction), 0.0), 32.0);
    let specular_color = specular_strength * light.color;

    let color = (ambient_color + diffuse_color + specular_color) * in.color.xyz;

    return vec4<f32>(color, in.color.a);
}