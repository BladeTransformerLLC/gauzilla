export function get_canvas_width() {
    var canvas = document.getElementById("render_canvas");
    var rect = canvas.getBoundingClientRect();
    return rect.width;
}


export function get_canvas_height() {
    var canvas = document.getElementById("render_canvas");
    var rect = canvas.getBoundingClientRect();
    return rect.height;
}


export function cpu_cores() {
    return navigator.hardwareConcurrency;
}


export function get_time_milliseconds() {
    return performance.now();
}


export function get_webgl1_version() {
    const gl = document.createElement("canvas").getContext("webgl");
    return gl.getParameter(gl.VERSION);
}


export function get_webgl2_version() {
    const gl = document.createElement("canvas").getContext("webgl2");
    return gl.getParameter(gl.VERSION);
}
