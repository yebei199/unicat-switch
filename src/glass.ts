// WebGL 合成：低分辨率场景按最近邻放大铺满胶囊轨道，玻璃球在上面做球面折射。
const VERT = 'attribute vec2 a; void main() { gl_Position = vec4(a, 0.0, 1.0); }';

const FRAG = `
precision highp float;
uniform sampler2D uScene;
uniform vec2 uRes;
uniform vec4 uBall;
uniform float uNight;

vec3 scene(vec2 p) { return texture2D(uScene, clamp(p / uRes, 0.0, 1.0)).rgb; }

float pill(vec2 p) {
  float r = uRes.y * 0.5;
  vec2 q = abs(p - uRes * 0.5) - vec2(uRes.x * 0.5 - r, 0.0);
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

// asin(r) 映射：球心放大约 1.57 倍，越到边缘压缩越狠，边界处与外部画面连续。
vec3 glass(vec2 d, float r) {
  float k = r > 1e-4 ? asin(r) / (1.5707963 * r) : 0.6366;
  vec2 off = d * uBall.zw * k;
  float ca = 0.014 * r * r;
  vec3 c = vec3(
    scene(uBall.xy + off * (1.0 + ca)).r,
    scene(uBall.xy + off).g,
    scene(uBall.xy + off * (1.0 - ca)).b);
  float z = sqrt(max(1.0 - r * r, 0.0));
  c = mix(c, vec3(0.96, 0.98, 1.0), pow(1.0 - z, 3.0) * mix(0.55, 0.12, uNight));
  c *= mix(1.0, 0.8, smoothstep(0.86, 1.0, r));
  vec2 h = (d - vec2(-0.36, -0.48)) * vec2(1.0, 1.8);
  c += smoothstep(0.36, 0.04, length(h)) * mix(0.62, 0.2, uNight);
  vec2 dir = d / max(r, 1e-4);
  float rim = smoothstep(0.76, 0.97, r) * smoothstep(0.2, 1.0, dot(dir, normalize(vec2(0.55, 0.83))));
  return c + rim * mix(0.4, 0.14, uNight);
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y - gl_FragCoord.y);
  float sd = pill(p);
  float mask = clamp(0.5 - sd, 0.0, 1.0);
  vec3 c = scene(p);
  vec2 sp = (p - uBall.xy - vec2(0.0, uBall.w * 0.12)) / (uBall.zw * 1.04);
  c *= 1.0 - 0.3 * smoothstep(1.25, 0.9, length(sp));
  float inner = smoothstep(-uRes.y * 0.1, 0.0, sd);
  c *= 1.0 - inner * (0.22 + 0.25 * smoothstep(uRes.y * 0.45, 0.0, p.y));
  vec2 d = (p - uBall.xy) / uBall.zw;
  float r = length(d);
  float edge = clamp((1.0 - r) * uBall.z, 0.0, 1.0);
  if (edge > 0.0) c = mix(c, glass(d, min(r, 0.999)), edge);
  gl_FragColor = vec4(c * mask, mask);
}`;

/** 球心与两个半轴，单位是画布设备像素，原点左上。 */
export type Ball = { x: number; y: number; rx: number; ry: number };

function compile(gl: WebGLRenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error('createShader failed');
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader) ?? 'shader');
  return shader;
}

export class Glass {
  private readonly gl: WebGLRenderingContext;
  private readonly uniforms: Record<'res' | 'ball' | 'night', WebGLUniformLocation | null>;

  constructor(readonly canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl', { antialias: false, premultipliedAlpha: true });
    if (!gl) throw new Error('WebGL unavailable');
    this.gl = gl;
    const program = gl.createProgram();
    gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERT));
    gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAG));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? 'link');
    gl.useProgram(program);
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindTexture(gl.TEXTURE_2D, gl.createTexture());
    for (const [k, v] of [
      [gl.TEXTURE_MIN_FILTER, gl.NEAREST],
      [gl.TEXTURE_MAG_FILTER, gl.NEAREST],
      [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE],
      [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE],
    ] as const)
      gl.texParameteri(gl.TEXTURE_2D, k, v);
    this.uniforms = {
      res: gl.getUniformLocation(program, 'uRes'),
      ball: gl.getUniformLocation(program, 'uBall'),
      night: gl.getUniformLocation(program, 'uNight'),
    };
  }

  draw(scene: ImageData, ball: Ball, night: number): void {
    const { gl, canvas } = this;
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, scene);
    gl.uniform2f(this.uniforms.res, canvas.width, canvas.height);
    gl.uniform4f(this.uniforms.ball, ball.x, ball.y, ball.rx, ball.ry);
    gl.uniform1f(this.uniforms.night, night);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}
