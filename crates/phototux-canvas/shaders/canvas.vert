#version 440
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 texcoord;
layout(location = 0) out vec2 v_uv;

layout(std140, binding = 0) uniform buf {
    vec4 color;      // rgba document tint
    vec4 params;     // x=hasDoc (0/1), y=phase, z=unused, w=unused
};

void main()
{
    v_uv = texcoord;
    gl_Position = vec4(position, 0.0, 1.0);
}
