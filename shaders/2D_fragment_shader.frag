#version 450

layout(location = 0) in vec2 v_texCoords;
layout(location = 1) in vec4 v_outColor;

// Make sure set/binding match your setup (e.g., set=0, binding=0)
layout(set = 0, binding = 0) uniform sampler2D font_texture;

layout(location = 0) out vec4 f_outColor;

void main() {
    // Sample the texture and multiply by vertex color
    // (Texture often contains grayscale font shapes + alpha)
    f_outColor = v_outColor * (font_texture, v_texCoords);
}
