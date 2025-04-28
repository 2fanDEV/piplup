#version 450

layout(location = 1) in vec2 v_texCoords;
layout(set = 0, binding = 0) uniform sampler2D image_texture; // Use a name indicating image

layout(location = 0) out vec4 f_outColor;

void main() {
    vec4 sampled_data = texture(image_texture, v_texCoords);
    f_outColor = sampled_data;
}
