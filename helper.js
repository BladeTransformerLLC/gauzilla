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


var _floatView = new Float32Array(1);
var _int32View = new Int32Array(_floatView.buffer);
export function float2half(float) {
    _floatView[0] = float;
    var f = _int32View[0];

    var sign = (f >> 31) & 0x0001;
    var exp = (f >> 23) & 0x00ff;
    var frac = f & 0x007fffff;

    var newExp;
    if (exp == 0) {
        newExp = 0;
    } else if (exp < 113) {
        newExp = 0;
        frac |= 0x00800000;
        frac = frac >> (113 - exp);
        if (frac & 0x01000000) {
            newExp = 1;
            frac = 0;
        }
    } else if (exp < 142) {
        newExp = exp - 112;
    } else {
        newExp = 31;
        frac = 0;
    }

    return (sign << 15) | (newExp << 10) | (frac >> 13);
}


export function pack_half_2x16_js(x, y) {
    return (float2half(x) | (float2half(y) << 16)) >>> 0;
}
