import { onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import type { FrpcStatus } from "../types";

/**
 * 单次"波浪环"：一条带正弦扰动的闭合曲线（花瓣形圆环）。
 * - baseR 随时间向外扩散
 * - 花瓣相位随时间旋转（看起来像在转动）
 * - amp / lineWidth / alpha 随生命衰减
 * 多个 burst 错峰生成，叠加成"一层一层向外扩散"的水波纹。
 */
interface Burst {
  spawnTime: number;
  spread: number;       // 径向扩散速度 px/s
  maxR: number;         // 最大半径 px
  waveAmp: number;      // 圆周波浪幅度 px（让圆环变"花瓣"）
  waveCount: number;    // 圆周上的波数（花瓣数）
  waveFreq: number;     // 相位旋转角速度 rad/s（带正负，控制转向）
  phase: number;        // 初始相位
  r0: number;           // 起始半径（中心 0 / 边缘 BUTTON_RADIUS）
  color: string;        // 环颜色
  lineWidth: number;    // 基础线宽 px
}

const CONNECTED_COLORS = [
  "#ff6b9d", "#ffb800", "#00d4aa", "#5b7fff", "#c44fcb", "#ff8a3d",
];
const CONNECTING_COLORS = ["#ffb800", "#ff8a3d", "#ff6b9d", "#c44fcb"];

/** 按钮半径（与 .big-toggle 宽度一致：160/2），connecting 的环从此处起始 */
const BUTTON_RADIUS = 80;

/** 单个环圆周采样点数：太少呈多边形，太多浪费；96 足够平滑 */
const SAMPLE_N = 96;

function pickColor(palette: string[]): string {
  return palette[Math.floor(Math.random() * palette.length)];
}

function hexToRgb(hex: string): [number, number, number] {
  return [
    parseInt(hex.slice(1, 3), 16),
    parseInt(hex.slice(3, 5), 16),
    parseInt(hex.slice(5, 7), 16),
  ];
}

/**
 * Canvas 波浪环系统：
 * - connected：从按钮中心生成花瓣环，向外起伏扩散（多彩）
 * - connecting：从按钮边缘生成花瓣环，向外波动扩散（暖色）
 * - 其他状态：不生成新环，旧环自然消散后画布清空
 */
export function useParticles(status: Ref<FrpcStatus>) {
  const canvas = ref<HTMLCanvasElement | null>(null);

  let ctx: CanvasRenderingContext2D | null = null;
  const bursts: Burst[] = [];
  let raf: number | null = null;
  let lastFrame = 0;
  let spawnAcc = 0;
  let dpr = 1;

  function resize() {
    const el = canvas.value;
    if (!el || !ctx) return;
    const rect = el.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    el.width = Math.max(1, Math.floor(rect.width * dpr));
    el.height = Math.max(1, Math.floor(rect.height * dpr));
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function spawnBurst(now: number, palette: string[], origin: "center" | "edge") {
    bursts.push({
      spawnTime: now,
      spread: 50 + Math.random() * 30,      // 50-80 px/s
      maxR: 180 + Math.random() * 40,       // 180-220 px
      waveAmp: 16 + Math.random() * 16,    // 16-32 px（较大起伏）
      waveCount: 6 + Math.floor(Math.random() * 5), // 6-10 瓣
      waveFreq: (Math.random() < 0.5 ? -1 : 1) * (0.8 + Math.random() * 1.4), // ±0.8-2.2 rad/s
      phase: Math.random() * Math.PI * 2,
      // 起始半径 ±5 px 抖动：环不再严格同心，叠层时更错落
      r0: (origin === "center" ? 0 : BUTTON_RADIUS) + (Math.random() - 0.5) * 10,
      color: pickColor(palette),
      lineWidth: 1.6 + Math.random() * 1.4, // 1.6-3 px
    });
  }

  function drawBurst(burst: Burst, now: number, cx: number, cy: number) {
    if (!ctx) return;
    const t = (now - burst.spawnTime) / 1000;
    const baseR = Math.min(burst.r0 + t * burst.spread, burst.maxR);
    const lifeRatio = Math.max(0, 1 - baseR / burst.maxR);
    if (lifeRatio <= 0) return;

    // 花瓣幅度与相位：amp 随生命衰减并加时变呼吸（±20%），phase 随时间旋转
    const ampBreathe = 1 + Math.sin(t * 3.7 + burst.phase) * 0.2;
    const amp = burst.waveAmp * lifeRatio * ampBreathe;
    const phase = burst.phase + t * burst.waveFreq;

    // 极坐标采样 N+1 点（最后一帧回到起点保证闭合），连成波浪曲线
    // 双频叠加：基波 + 二次谐波（强度 0.3），让花瓣形状更不规则、环间差异更大
    // 归一化到 [0,1]，r 仍 ∈ [baseR, baseR+amp]
    ctx.beginPath();
    for (let i = 0; i <= SAMPLE_N; i++) {
      const theta = (i / SAMPLE_N) * Math.PI * 2;
      const w1 = (Math.sin(theta * burst.waveCount + phase) + 1) * 0.5;
      const w2 = (Math.sin(theta * (burst.waveCount * 2 + 1) + phase * 1.3) + 1) * 0.5;
      const wave = (w1 + w2 * 0.3) / 1.3;
      const r = baseR + wave * amp;
      const x = cx + r * Math.cos(theta);
      const y = cy + r * Math.sin(theta);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.closePath();

    const [rc, g, b] = hexToRgb(burst.color);
    ctx.strokeStyle = `rgba(${rc},${g},${b},${lifeRatio})`;
    ctx.lineWidth = burst.lineWidth * (0.5 + lifeRatio * 0.8);
    ctx.stroke();
  }

  function draw(now: number, cx: number, cy: number) {
    if (!ctx || !canvas.value) return;
    const rect = canvas.value.getBoundingClientRect();
    ctx.clearRect(0, 0, rect.width, rect.height);
    // 叠加模式：重叠环加亮，水波纹层叠效果
    ctx.globalCompositeOperation = "lighter";
    for (const b of bursts) drawBurst(b, now, cx, cy);
    ctx.globalCompositeOperation = "source-over";
  }

  function step(now: number) {
    if (!canvas.value) {
      raf = requestAnimationFrame(step);
      return;
    }
    if (!ctx) {
      ctx = canvas.value.getContext("2d");
      if (ctx) resize();
    }

    if (!lastFrame) lastFrame = now;
    const dt = Math.min(0.05, (now - lastFrame) / 1000);
    lastFrame = now;

    const s = status.value;
    if (s === "connected" || s === "connecting") {
      // 环生成速率（connected 更密）；环寿命 ≈ maxR/spread ≈ 3-4s
      const rate = s === "connected" ? 1.6 : 1.1;
      spawnAcc += dt * rate;
      const palette = s === "connected" ? CONNECTED_COLORS : CONNECTING_COLORS;
      const origin = s === "connected" ? "center" : "edge";
      while (spawnAcc >= 1) {
        spawnBurst(now, palette, origin);
        spawnAcc -= 1;
      }
    } else {
      spawnAcc = 0;
    }

    // 清理过期环（半径已达 maxR）
    for (let i = bursts.length - 1; i >= 0; i--) {
      const t = (now - bursts[i].spawnTime) / 1000;
      if (t > bursts[i].maxR / bursts[i].spread + 0.1) {
        bursts.splice(i, 1);
      }
    }

    const rect = canvas.value.getBoundingClientRect();
    const cx = rect.width / 2;
    const cy = rect.height / 2;
    draw(now, cx, cy);

    raf = requestAnimationFrame(step);
  }

  function clearAll() {
    bursts.length = 0;
  }

  watch(status, (s) => {
    if (s !== "connected" && s !== "connecting") clearAll();
  });

  onMounted(() => {
    if (!canvas.value) return;
    ctx = canvas.value.getContext("2d");
    resize();
    window.addEventListener("resize", resize);
    raf = requestAnimationFrame(step);
  });

  onBeforeUnmount(() => {
    if (raf) cancelAnimationFrame(raf);
    window.removeEventListener("resize", resize);
  });

  return { canvas };
}
