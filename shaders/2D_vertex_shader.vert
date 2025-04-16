#version 450

layout(location = 0) in vec2 a_inPos;
layout(location = 1) in vec2 a_texCoords;
layout(location = 2) in vec4 a_inColor;

layout(location = 0) out vec2 v_texCoords;
layout(location = 1) out vec4 v_outColor;

layout(push_constant) uniform PushConstant {
	mat4 screenToClip;
} pc;

void main() {
	gl_Position = pc.screenToClip * vec4(a_inPos, 0.0, 1.0);
	
	v_texCoords = a_texCoords;
	v_outColor = a_inColor;
}
