#version 440
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    vec4 color;
    vec4 params;     // x=hasDoc, y=phase, z=importOk, w=selectionAnts
};
layout(binding = 1) uniform sampler2D documentTexture;
layout(binding = 2) uniform sampler2D selectionTexture;

void main()
{
    vec4 doc = texture(documentTexture, v_uv);
    fragColor = doc;
    if (params.w < 0.5)
        return;

    float s = texture(selectionTexture, v_uv).r;
    ivec2 sz = textureSize(selectionTexture, 0);
    float texelW = 1.0 / float(max(sz.x, 1));
    float texelH = 1.0 / float(max(sz.y, 1));
    float n = texture(selectionTexture, v_uv + vec2(0.0, -texelH)).r;
    float e = texture(selectionTexture, v_uv + vec2(texelW, 0.0)).r;
    float so = texture(selectionTexture, v_uv + vec2(0.0, texelH)).r;
    float w = texture(selectionTexture, v_uv + vec2(-texelW, 0.0)).r;
    bool sel = s > 0.5;
    bool edge = sel != (n > 0.5) || sel != (e > 0.5) || sel != (so > 0.5) || sel != (w > 0.5);
    if (!edge)
        return;

    float stripe = floor(mod((v_uv.x + v_uv.y) * 80.0 + params.y * 10.0, 2.0));
    vec3 ant = mix(vec3(0.0), vec3(0.35, 0.62, 0.96), stripe);
    fragColor = vec4(mix(doc.rgb, ant, 1.0), doc.a);
}
