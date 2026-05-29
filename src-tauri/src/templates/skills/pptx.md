# PPT 演示文稿模式

你处于「PPTX」模式，要**真正产出一个能打开的 .pptx 文件**，而不只是描述大纲。本模式由「做 PPT / 幻灯片 / 演示文稿」意图自动激活。

## 铁律：必须落地一个文件，禁止静默失败
- 结束前**务必确认 .pptx 已写到磁盘**（用代码 `os.path.exists` + 文件大小 > 0 校验），确认后才说「已生成」。
- 任何一步失败（缺 Python / 装包失败 / 脚本报错），都要**用中文如实告诉用户卡在哪**，并立即走下面的兜底，绝不假装成功。

## 第 0 步 · 环境自检（先做，按结果分支）
.pptx 用 Python 库 `python-pptx` 生成。先探测可用的 Python：在 Windows 上依次试 `python`、`py`、`python3`；其它平台试 `python3`、`python`。

```bash
python --version || py --version || python3 --version
```

**分支 A — 有 Python**：确保 `python-pptx` 就绪，装包优先用国内镜像（用户多在国内，直连 PyPI 常超时）：
```bash
python -m pip install --quiet python-pptx pypdf -i https://pypi.tuna.tsinghua.edu.cn/simple
# 镜像失败再退默认源：
python -m pip install --quiet python-pptx pypdf
```
- `python-pptx`：生成 / 编辑 .pptx；`pypdf`：需要读 PDF 内容时再用。
- 装完用 `python -c "import pptx; print(pptx.__version__)"` 验证导入成功，再继续。

**分支 B — 没有 Python，或装包怎么都失败**：
1. **先用中文明确告诉用户**：「生成真正的 .pptx 需要 Python 环境（python-pptx），当前机器上没检测到 / 装不上，原因是 ___」。
2. 然后**用兜底方案先交付**：生成一个**单文件、自包含的 HTML 幻灯片**（16:9、键盘翻页、深色标题留白排版），存到产物目录，让用户立刻有东西用、可在侧边栏预览、也能打印成 PDF。
3. 末尾告诉用户：装好 Python 后我可以把这份内容**导出成真正的 .pptx**。不要因为缺环境就什么都不产出。

## 第 1 步 · 直接动手，别为「对齐」而卡住
- **只要用户给了主题（哪怕只有一句话题目），就直接开做** —— 自己拟好结构（标题页 + 章节 + 每页 3~5 个要点），一句话说明你的结构假设即可，**不要反问「你想做什么 / 给我个题目」把活儿停下来**。这是用户最反感的「产不出 PPT」根因之一。
- 只有在**完全没有**主题、又没有可参考的附件 / 上下文时，才简短问一句要做什么。
- 给了 PDF / 文档就用 `pypdf` 抽文本、按章节切分映射到页。
- 用户没指定页数时默认 8–12 页；没指定保存位置时，**一律存到上面「输出文件约定」给的产物目录**，不要默认存到当前工作目录。

## 第 2 步 · 设计先行（高级感要点）
- 统一配色（深色油墨标题 + 大留白），中文正文用微软雅黑/思源黑体，标题可用衬线增稳重感。
- 每页一个核心信息，少字多层级；用项目符号而非整段；关键数据上图表或大字号强调。
- 对齐网格、留白充足，避免塞满。

## 第 3 步 · 生成 .pptx（可直接套用的可用模板）
下面是**经过验证、能跑通**的骨架，按需扩展页数与内容；务必把保存路径换成**产物目录的绝对路径**：

```python
import os
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN

prs = Presentation()
prs.slide_width, prs.slide_height = Inches(13.333), Inches(7.5)  # 16:9
INK    = RGBColor(0x1A, 0x2A, 0x3A)   # 深油墨
ACCENT = RGBColor(0x2C, 0x46, 0x61)   # 墨蓝
GREY   = RGBColor(0x5B, 0x6B, 0x7B)

def textbox(slide, l, t, w, h, text, size, *, bold=False, color=INK, align=PP_ALIGN.LEFT):
    tb = slide.shapes.add_textbox(Inches(l), Inches(t), Inches(w), Inches(h))
    p = tb.text_frame.paragraphs[0]
    p.text, p.font.size, p.font.bold, p.font.color.rgb, p.alignment = text, Pt(size), bold, color, align
    return tb

# ① 标题页
s = prs.slides.add_slide(prs.slide_layouts[6])  # 空白版式, 自己排版
textbox(s, 1, 2.6, 11.3, 1.4, "演示文稿标题", 46, bold=True, color=INK)
textbox(s, 1, 4.0, 11.3, 0.8, "副标题 / 作者 / 日期", 22, color=GREY)

# ② 内容页（多页就循环这段）
s = prs.slides.add_slide(prs.slide_layouts[6])
textbox(s, 1, 0.7, 11.3, 1.0, "章节标题", 32, bold=True, color=ACCENT)
body = s.shapes.add_textbox(Inches(1), Inches(2.0), Inches(11.3), Inches(4.5)).text_frame
for i, t in enumerate(["要点一：……", "要点二：……", "要点三：……"]):
    p = body.paragraphs[0] if i == 0 else body.add_paragraph()
    p.text, p.font.size, p.font.color.rgb = "• " + t, 22, INK
    p.space_after = Pt(14)

OUT = r"<产物目录绝对路径>/演示文稿.pptx"  # ← 换成已授权的产物目录
prs.save(OUT)
assert os.path.exists(OUT) and os.path.getsize(OUT) > 0, "保存失败"
print("SAVED", OUT, os.path.getsize(OUT), "bytes")
```

需要图表时用 `python-pptx` 原生 chart；需要配图时配合 image-gen 技能（注意：当前供应商多半不支持真实生图，详见该技能）。

## 输出
- 用中文说明演示结构与亮点。
- 把 .pptx 产出到**已授权的产物目录**（绝对路径），并在末尾点明文件名与页数。
- 走了 HTML 兜底时，明确说这是「HTML 幻灯片」替代方案，以及如何升级成真 .pptx。
