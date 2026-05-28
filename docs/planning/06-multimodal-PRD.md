# 板块 ⑥ 多模态输入 · 规划 PRD (v0.3+)

> 状态: **规划中** (MVP v0.1 未实现)
> 上游 PRD: `PRD-v6.html` §12
> 优先级: P1

## 一、板块边界

**做**:
- 文件转 Markdown (PDF / Word / PPT / Excel / HTML / img-OCR)
- 图片转 AI 可读 (base64 / 描述)
- 语音输入 (本地实时转写) — **要做到行业领先**, 参考 GitHub 开源项目

**不做**:
- 发消息 (① 对话核心)
- 持久化 (② Wiki storage)

## 二、文件转 MD 路径选型

| 文件类型 | 工具 | 备注 |
|---------|------|------|
| PDF | `marker` / `pdf2htmlEX` + 后处理 | 章节/表格/公式;扫描件走 PaddleOCR |
| Word .docx | `pandoc` | 样式映射(标题/列表/表格/脚注) |
| PowerPoint | 解析 OOXML + 每页转图 + OCR | 每页 = 一个 ## 标题 + 文字 + 图(base64) |
| Excel | `calamine` (Rust 原生) | 每 sheet 转 MD 表格 |
| HTML | `readability` + `turndown` | 抽正文 + 清洗 |
| 图片 | PaddleOCR / Tesseract / Vision 直发 | |
| 音视频 | 复用语音子模块 | 带时间戳的 MD |

## 三、语音输入 (重点)

### 双引擎可切换

- **FunASR (Paraformer-zh-streaming)** — 中文默认, 阿里达摩
- **whisper.cpp (large-v3-turbo)** — 英文 / 多语

### UX 流程 (豆包模式,见用户偏好)

```
用户长按「● 语音输入」按钮
  → 开始捕获麦克风, VAD 检测说话
  → 输入框上方浮录音条 [●●●● ⌬ 电平]
  → 实时 partial 灰色显示, 句末标点 + 确认 → 文本变黑
  → 检测到指令词「发送 / 换行 / 撤销最后一句」→ 对应动作
  → 松手 / 静默 5s → voice_stop → finalize
```

### 公开 API

```rust
pub fn file_to_md(path: &Path, opts: FileToMdOpts) -> Result<MdResult>;
pub fn image_to_ai(path: &Path, mode: ImgMode) -> Result<AiImage>;

pub fn voice_engines() -> Vec<VoiceEngine>;
pub async fn voice_start(engine: VoiceEngine, opts: VoiceOpts) -> Result<Receiver<Partial>>;
pub async fn voice_stop() -> Result<String /* final */>;
pub fn voice_devices() -> Vec<AudioDevice>;
pub fn voice_hotwords_set(words: Vec<String>) -> Result<()>;
```

## 四、参考开源项目

| 项目 | 用途 |
|------|------|
| `modelscope/FunASR` | ASR 引擎 (Paraformer 流式 + CT-Transformer 标点) |
| `ggerganov/whisper.cpp` | ASR 引擎 (Ggml 推理) |
| `snakers4/silero-vad` | VAD |
| `xiangsx/whisper-record-app` | 录音 UX |
| `VikParuchuri/marker` | PDF → MD |
| `jgm/pandoc` | 多格式转换 |
| `PaddlePaddle/PaddleOCR` | OCR |

## 五、里程碑

- v0.3: 文件转 MD (Excel + HTML + .docx via pandoc)
- v0.4: 图片转 AI (Vision 直传)
- v0.5: 语音输入 (云端首发, 豆包模式 UX)
- v0.6: 本地 FunASR / whisper.cpp (用户隐私)
