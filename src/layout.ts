// 低分辨率画布与 art/scene.png 里各物件的坐标（单位：场景像素），由素材实测得出。
export const W = 224;
export const H = 96;
export const HORIZON = 66;
export const WINDOW = { x0: 25, y0: 55, x1: 41, y1: 72 } as const;
export const DOOR = { x: 75, y: 84 } as const;
export const TREE = { x: 164, y: 86 } as const;
export const GLOW = { x: 56, y: 85, rx: 34, ry: 9 } as const;
