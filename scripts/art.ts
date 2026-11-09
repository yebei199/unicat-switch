// Codex 原图（art/src，品红底）→ 低分辨率像素素材（art/*.png）：抠底、按面积缩小、硬化透明边、压色板。
import { $ } from 'bun';

const SRC = 'art/src';
const OUT = 'art';
const PALETTE = `${OUT}/.cat-palette.png`;
const KEY = ['-fuzz', '20%', '-transparent', '#ff00ff'];
const TRIM = ['-trim', '+repage'];
const HARD = ['-channel', 'A', '-threshold', '50%', '+channel'];
// 猫在 18px 高时只剩色块，先拉对比再映射到固定色板，白围兜和白爪子才不会被平均成灰。
const CAT_COLORS = ['#140d0b', '#2e1f1a', '#4d3529', '#f3eee6', '#b9b2a8', '#e6c33c', '#d99aa1'];
const CAT = ['-sigmoidal-contrast', '4x40%', '-filter', 'Box'];
const REMAP = ['-dither', 'None', '-remap', PALETTE];

await $`magick ${CAT_COLORS.map((c) => `xc:${c}`)} xc:none +append ${PALETTE}`;
await $`magick ${SRC}/scene.png ${KEY} -filter Box -resize 224x96^ -background none -gravity center -extent 224x96 ${HARD} -colors 48 ${OUT}/scene.png`;
await $`magick ${SRC}/cat-sit.png ${KEY} ${TRIM} ${CAT} -resize x18 ${HARD} ${REMAP} ${OUT}/cat-sit.png`;
// 走路图是一行等宽 4 帧：整体裁边后等分再缩放，四帧同宽同高、脚底对齐。
await $`magick ${SRC}/cat-walk.png ${KEY} ${TRIM} -crop 4x1@ +repage ${CAT} -resize x15 ${HARD} +append ${REMAP} ${OUT}/cat-walk.png`;
await $`rm ${PALETTE}`;
