#version 440
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    vec4 color;      // rgba document tint
    vec4 params;     // x=hasDoc, y=phase
};

void main()
{
    // GPU-only checker: no CPU pixel upload (ADR-005).
    vec2 cells = floor(v_uv * vec2(24.0, 16.0));
    float checker = mod(cells.x + cells.y, 2.0);
    vec3 base = color.rgb;
    vec3 dark = base * 0.82;
    vec3 lit = mix(dark, base, checker);
    // Subtle phase pulse so continuous frames exercise the GPU path.
    float pulse = 0.92 + 0.08 * (0.5 + 0.5 * sin(params.y));
    fragColor = vec4(lit * pulse, color.a);
}
