#version 450



layout(location = 0) in vec2 v_texCoords;
layout(location = 1) in vec4 v_outColor;

layout(set = 0, binding = 0) uniform sampler2D font_texture;

layout(location = 0) out vec4 f_outColor;

void main() {
    vec4 sampled_data = texture(font_texture, v_texCoords);
    float font_shape_alpha = sampled_data.r * v_outColor.a; 
    f_outColor = vec4(v_outColor.rgb, font_shape_alpha);
}
