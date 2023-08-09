precision mediump float;

attribute vec3 a_position;
attribute vec4 a_color;

 varying lowp vec4 color;

void main() {
    color = a_color;
    gl_Position = vec4(a_position, 1.0);
}
