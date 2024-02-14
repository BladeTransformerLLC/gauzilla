#version 300 es
precision highp float;

uniform sampler2D u_screen_texture;

in vec2 texcoords;

out vec4 fragColor;

void main() {
    fragColor = texture(u_screen_texture, texcoords);
}
