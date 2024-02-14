#version 300 es
precision highp float;

in vec3 position;
out vec2 texcoords;

void main() {
    gl_Position = vec4(position, 1.0);
    texcoords = 0.5*position.xy + 0.5;
}
