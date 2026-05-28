<script setup lang="ts">
defineProps<{ busy?: boolean }>();
</script>

<template>
  <div class="worker" :class="busy ? 'busy' : 'idle'">
    <svg viewBox="0 0 120 120" class="fig">
      <!-- 案几 -->
      <rect x="18" y="86" width="84" height="9" rx="2" class="desk" />
      <rect x="22" y="95" width="6" height="16" class="leg" />
      <rect x="92" y="95" width="6" height="16" class="leg" />

      <!-- 案上卷轴/纸 -->
      <rect x="40" y="80" width="40" height="7" rx="1.5" class="scroll" />

      <!-- 身体 · 长袍 -->
      <g class="body-sway">
        <path class="robe" d="M42 86 Q44 58 60 56 Q76 58 78 86 Z" />
        <!-- 衣领交叠 -->
        <path class="collar" d="M54 60 L60 70 L66 60" />
        <!-- 头 -->
        <circle class="face" cx="60" cy="46" r="11" />
        <!-- 发髻 + 簪 -->
        <path class="hair" d="M49 44 Q50 33 60 32 Q70 33 71 44 Q66 39 60 39 Q54 39 49 44 Z" />
        <circle class="bun" cx="60" cy="30" r="4.5" />
        <rect class="pin" x="59" y="24" width="2" height="7" rx="1" />
        <!-- 眉眼（含蓄古风细线） -->
        <path class="brow" d="M54 45 q2 -1.5 4 0" />
        <path class="brow" d="M62 45 q2 -1.5 4 0" />

        <!-- 手臂（执笔 / 搁案） -->
        <g class="arm">
          <path class="sleeve" d="M58 66 Q66 70 72 78" />
          <!-- 毛笔（忙时显示） -->
          <g class="brush">
            <rect x="71" y="68" width="2.4" height="12" rx="1" />
            <path class="brush-tip" d="M72.2 80 l-1.6 5 l3.2 0 Z" />
          </g>
        </g>
      </g>

      <!-- 油灯（忙时亮） -->
      <g class="lamp">
        <rect x="26" y="74" width="10" height="6" rx="1.5" class="lamp-base" />
        <circle class="flame" cx="31" cy="71" r="3.2" />
      </g>

      <!-- 摸鱼：浮鱼（闲时显示） -->
      <g class="fish">
        <ellipse cx="92" cy="64" rx="7" ry="4" />
        <path d="M99 64 l6 -4 l0 8 Z" />
        <circle cx="89" cy="63" r="0.9" class="fish-eye" />
      </g>

      <!-- 打盹 Zzz（闲时显示） -->
      <text class="zzz z1" x="72" y="40">z</text>
      <text class="zzz z2" x="78" y="33">z</text>
      <text class="zzz z3" x="84" y="27">z</text>
    </svg>
  </div>
</template>

<style scoped>
.worker {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: flex-end;
  justify-content: center;
}
.fig {
  width: 100%;
  height: 100%;
  overflow: visible;
}

/* 配色：墨蓝长袍 + 暖肤 + 朱砂细节 */
.desk { fill: #b08a5a; }
.leg { fill: #8d6c43; }
.scroll { fill: #f3efe4; stroke: #d8cdb0; stroke-width: 0.6; }
.robe { fill: #2c4661; }
.collar { fill: none; stroke: #d8e0ea; stroke-width: 1.6; stroke-linecap: round; }
.face { fill: #f0d9bf; }
.hair { fill: #20242c; }
.bun { fill: #20242c; }
.pin { fill: #a78c4f; }
.brow { fill: none; stroke: #3a3a40; stroke-width: 1; stroke-linecap: round; }
.sleeve { fill: none; stroke: #2c4661; stroke-width: 6; stroke-linecap: round; }
.brush rect { fill: #6b4a2b; }
.brush-tip { fill: #20242c; }
.lamp-base { fill: #8d6c43; }
.flame { fill: #f0a020; }

/* 身体轻微摆动 */
.body-sway { transform-box: fill-box; transform-origin: 60px 86px; }
.idle .body-sway { animation: sway 4.2s ease-in-out infinite; }
.busy .body-sway { animation: focus 1.1s ease-in-out infinite; }
@keyframes sway {
  0%, 100% { transform: rotate(-2deg); }
  50% { transform: rotate(2deg); }
}
@keyframes focus {
  0%, 100% { transform: translateY(0) rotate(0); }
  50% { transform: translateY(1px) rotate(0.5deg); }
}

/* 执笔手臂：忙时来回书写 */
.arm { transform-box: fill-box; transform-origin: 58px 66px; }
.busy .arm { animation: write 0.9s ease-in-out infinite; }
@keyframes write {
  0%, 100% { transform: translate(0, 0); }
  50% { transform: translate(-5px, 1px); }
}

/* 毛笔/油灯仅忙时显示 */
.brush, .lamp { opacity: 0; transition: opacity 0.3s; }
.busy .brush, .busy .lamp { opacity: 1; }
.busy .flame { animation: flicker 0.6s ease-in-out infinite alternate; transform-box: fill-box; transform-origin: 31px 74px; }
@keyframes flicker {
  from { transform: scaleY(0.85); opacity: 0.85; }
  to { transform: scaleY(1.1); opacity: 1; }
}

/* 摸鱼之鱼 + Zzz 仅闲时显示 */
.fish { fill: #7fae9b; opacity: 0; }
.fish-eye { fill: #20242c; }
.idle .fish { opacity: 1; animation: swim 3s ease-in-out infinite; transform-box: fill-box; transform-origin: 95px 64px; }
@keyframes swim {
  0%, 100% { transform: translateY(0) rotate(-3deg); }
  50% { transform: translateY(-5px) rotate(3deg); }
}
.zzz {
  font-family: var(--serif);
  fill: #8aa0c0;
  opacity: 0;
}
.z1 { font-size: 9px; }
.z2 { font-size: 7px; }
.z3 { font-size: 5px; }
.idle .zzz { animation: zfloat 3s ease-in-out infinite; }
.idle .z2 { animation-delay: 0.4s; }
.idle .z3 { animation-delay: 0.8s; }
@keyframes zfloat {
  0% { opacity: 0; }
  40% { opacity: 0.9; }
  100% { opacity: 0; }
}
</style>
