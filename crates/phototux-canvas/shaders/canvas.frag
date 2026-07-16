#version 440
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    vec4 color;
    vec4 params;     // x=hasDoc, y=phase, z=importOk
};
layout(binding = 1) uniform sampler2D documentTexture;

void main()
{
    fragColor = texture(documentTexture, v_uv);
}
