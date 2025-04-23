#version 450



layout(location = 0) in vec2 v_texCoords;
layout(location = 1) in vec4 v_outColor;

// Make sure set/binding match your setup (e.g., set=0, binding=0)
layout(set = 0, binding = 0) uniform sampler2D font_texture;

layout(location = 0) out vec4 f_outColor;

void main() {
    vec4 sampled_data = texture(font_texture, v_texCoords);
    float font_shape_alpha = sampled_data.r * v_outColor.a; // Opacity from font atlas (0.0 to 1.0)

    // v_outColor is the desired text color (e.g., gray [80, 80, 80, 255])
    // Output the desired color (RGB from v_outColor) with the opacity from the font shape
    f_outColor = vec4(v_outColor.rgb, font_shape_alpha);
}
