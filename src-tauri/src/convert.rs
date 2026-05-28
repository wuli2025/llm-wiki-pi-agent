//! 文件 → Markdown / 纯文本 转换 (板块② 上传与入库共用)
//!
//! clean-room 自写。架构思想（按扩展名分发、Office 文件 = zip+xml、表格→markdown、
//! 用 catch_unwind 在命令边界兜住第三方解析库的 panic）系通用做法独立实现，
//! 未引用任何 GPL 项目的源码。
//!
//! 入口 [`convert_to_markdown`]：
//!   - `Ok(Some(md))` → 抽出了文本，调用方写成 `.md`
//!   - `Ok(None)`     → 无需 / 无法抽文本（图片、音视频、压缩包、未知二进制），
//!                      调用方应原样复制文件
//!   - `Err(msg)`     → 解析失败（含被 guard 兜住的 panic）

use std::io::Read;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

/// 文本类读取上限：避免一个超大文件撑爆内存与索引（20 MB）。
const TEXT_READ_CAP: u64 = 20 * 1024 * 1024;
/// 表格每个 sheet 最多渲染的行数，超出截断并注明。
const SHEET_ROW_CAP: usize = 2000;

/// panic 兜底：把第三方解析库（pdf-extract / calamine …）的 panic 转成 Err。
/// 依赖 release profile `panic = "unwind"`（见 Cargo.toml）。
fn guard<T>(label: &str, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "未知错误".into());
            Err(format!("{label} 解析失败（已隔离）: {msg}"))
        }
    }
}

/// 直接当纯文本读取的扩展名（含常见配置 / 代码 / 标记语言）。
const TEXT_EXTS: &[&str] = &[
    // 文档 / 标记
    "txt", "text", "log", "md", "markdown", "mdx", "rst", "org", "tex",
    // 数据 / 配置
    "csv", "tsv", "json", "jsonl", "ndjson", "yaml", "yml", "toml", "ini", "conf", "cfg",
    "env", "properties", "xml", "html", "htm", "svg",
    // 代码
    "rs", "js", "mjs", "cjs", "ts", "tsx", "jsx", "vue", "py", "go", "java", "kt", "kts",
    "swift", "c", "h", "cpp", "cc", "cxx", "hpp", "cs", "rb", "php", "pl", "lua", "sh",
    "bash", "zsh", "fish", "bat", "ps1", "psm1", "sql", "graphql", "gql", "r", "scala",
    "dart", "ex", "exs", "erl", "clj", "hs", "ml", "jl", "vim", "css", "scss", "sass",
    "less", "styl", "makefile", "dockerfile", "gradle", "proto",
];

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// 转换分发。见模块文档说明三种返回。
pub fn convert_to_markdown(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Err("文件不存在".into());
    }
    let ext = ext_of(path);

    // 无扩展名也尝试当文本（README、LICENSE、Makefile 等）
    if ext.is_empty() || TEXT_EXTS.contains(&ext.as_str()) {
        return read_text(path).map(Some);
    }

    match ext.as_str() {
        "docx" => guard("Word 文档", || extract_docx(path)).map(Some),
        "pptx" => guard("PPT 演示", || extract_pptx(path)).map(Some),
        "xlsx" | "xlsm" | "xlsb" | "xls" | "ods" => {
            guard("表格", || extract_spreadsheet(path)).map(Some)
        }
        "pdf" => guard("PDF", || extract_pdf(path)).map(Some),
        // 图片 / 音视频 / 压缩包 / 未知二进制：不抽文本，调用方原样保存
        _ => Ok(None),
    }
}

// ───────────────────────── 纯文本 ─────────────────────────

fn read_text(path: &Path) -> Result<String, String> {
    let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let bytes = std::fs::read(path).map_err(|e| format!("读取失败: {e}"))?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if len > TEXT_READ_CAP {
        // from_utf8_lossy 已读全部；这里仅在超大时截断展示，避免索引膨胀
        let cap = byte_floor_char_boundary(&text, TEXT_READ_CAP as usize);
        text.truncate(cap);
        text.push_str("\n\n…（文件过大，已截断）");
    }
    Ok(text)
}

fn byte_floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

// ───────────────────────── docx ─────────────────────────

fn open_zip(path: &Path) -> Result<zip::ZipArchive<std::fs::File>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("打开失败: {e}"))?;
    zip::ZipArchive::new(file).map_err(|e| format!("非法压缩包: {e}"))
}

fn read_zip_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Result<String, String> {
    let mut f = zip
        .by_name(name)
        .map_err(|_| format!("缺少 {name}"))?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

fn extract_docx(path: &Path) -> Result<String, String> {
    let mut zip = open_zip(path)?;
    let xml = read_zip_entry(&mut zip, "word/document.xml")?;
    let text = ooxml_text(&xml);
    if text.trim().is_empty() {
        return Err("未抽取到正文".into());
    }
    Ok(squeeze_blank_lines(&text))
}

// ───────────────────────── pptx ─────────────────────────

fn extract_pptx(path: &Path) -> Result<String, String> {
    let mut zip = open_zip(path)?;
    // 收集 ppt/slides/slideN.xml 并按 N 排序
    let mut slides: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slides.sort_by_key(|n| slide_index(n));

    let mut out = String::new();
    for (i, name) in slides.iter().enumerate() {
        let xml = read_zip_entry(&mut zip, name)?;
        let text = ooxml_text(&xml);
        if !out.is_empty() {
            out.push_str("\n\n---\n\n");
        }
        out.push_str(&format!("## 幻灯片 {}\n\n", i + 1));
        out.push_str(text.trim());
    }
    if out.trim().is_empty() {
        return Err("未抽取到内容".into());
    }
    Ok(squeeze_blank_lines(&out))
}

fn slide_index(name: &str) -> u32 {
    name.trim_end_matches(".xml")
        .rsplit("slide")
        .next()
        .and_then(|d| d.parse().ok())
        .unwrap_or(0)
}

// ───────────────────────── OOXML 文本抽取 ─────────────────────────

/// 把 OOXML（docx/pptx）的 XML 片段还原成纯文本：
/// 段落 `</w:p>` / `</a:p>` → 换行；制表 / 换行标签 → 对应空白；其余标签全部剥离；
/// 再解码 XML 实体。简单稳健，不依赖完整 XML 解析器。
fn ooxml_text(xml: &str) -> String {
    let mut s = xml.to_string();
    for close in ["</w:p>", "</a:p>"] {
        s = s.replace(close, "\n");
    }
    for br in ["<w:br/>", "<w:br />", "<a:br/>", "<a:br />"] {
        s = s.replace(br, "\n");
    }
    for tab in ["<w:tab/>", "<w:tab />", "<a:tab/>", "<a:tab />"] {
        s = s.replace(tab, "\t");
    }
    let stripped = strip_tags(&s);
    decode_xml_entities(&stripped)
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_xml_entities(s: &str) -> String {
    // &amp; 最后解码，避免二次解码
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
}

fn squeeze_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank_run = 0;
    for line in s.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

// ───────────────────────── 表格 (calamine) ─────────────────────────

fn extract_spreadsheet(path: &Path) -> Result<String, String> {
    use calamine::{open_workbook_auto, Reader};

    let mut wb = open_workbook_auto(path).map_err(|e| format!("打开表格失败: {e}"))?;
    let names = wb.sheet_names().to_owned();
    if names.is_empty() {
        return Err("空工作簿".into());
    }

    let mut out = String::new();
    for name in names {
        let range = match wb.worksheet_range(&name) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if range.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("## {name}\n\n"));

        let mut rows = range.rows();
        let header = rows.next();
        let ncol = header.map(|h| h.len()).unwrap_or(0).max(1);

        // 表头（无表头则用占位列名）
        let header_cells: Vec<String> = match header {
            Some(h) => h.iter().map(|c| md_cell(&cell_to_string(c))).collect(),
            None => (1..=ncol).map(|i| format!("列{i}")).collect(),
        };
        out.push_str(&format!("| {} |\n", header_cells.join(" | ")));
        out.push_str(&format!("|{}\n", " --- |".repeat(ncol.max(header_cells.len()))));

        let mut count = 0usize;
        for row in rows {
            if count >= SHEET_ROW_CAP {
                out.push_str(&format!("\n_（仅显示前 {SHEET_ROW_CAP} 行）_\n"));
                break;
            }
            let cells: Vec<String> = row.iter().map(|c| md_cell(&cell_to_string(c))).collect();
            out.push_str(&format!("| {} |\n", cells.join(" | ")));
            count += 1;
        }
    }

    if out.trim().is_empty() {
        return Err("无可读内容".into());
    }
    Ok(out.trim().to_string())
}

fn cell_to_string(d: &calamine::Data) -> String {
    use calamine::Data;
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => trim_float(*f),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        other => format!("{other:?}"),
    }
}

fn trim_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        f.to_string()
    }
}

/// 转义 markdown 表格单元格：竖线与换行会破坏表格结构。
fn md_cell(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('\r', " ")
        .replace('\n', " ")
        .trim()
        .to_string()
}

// ───────────────────────── PDF ─────────────────────────

fn extract_pdf(path: &Path) -> Result<String, String> {
    let text = pdf_extract::extract_text(path).map_err(|e| format!("PDF 抽取失败: {e}"))?;
    if text.trim().is_empty() {
        return Err("未抽取到文本（可能是扫描件 / 纯图片 PDF）".into());
    }
    Ok(squeeze_blank_lines(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docx_paragraphs_become_newlines() {
        let xml = "<w:p><w:r><w:t>第一段</w:t></w:r></w:p>\
                   <w:p><w:r><w:t>第二段</w:t></w:r></w:p>";
        let out = ooxml_text(xml);
        assert_eq!(out.trim(), "第一段\n第二段");
    }

    #[test]
    fn ooxml_handles_tabs_breaks_entities() {
        let xml = "<w:p><w:r><w:t>a</w:t><w:tab/><w:t>b</w:t><w:br/><w:t>c &amp; d</w:t></w:r></w:p>";
        let out = ooxml_text(xml);
        assert_eq!(out.trim(), "a\tb\nc & d");
    }

    #[test]
    fn entities_decoded_amp_last() {
        // &amp;lt; 必须只解一层 → "&lt;"，不能变成 "<"
        assert_eq!(decode_xml_entities("x &amp;lt; y"), "x &lt; y");
        assert_eq!(decode_xml_entities("&lt;tag&gt; &quot;q&quot;"), "<tag> \"q\"");
    }

    #[test]
    fn md_cell_escapes_pipe_and_newline() {
        assert_eq!(md_cell("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn slide_index_parses_number() {
        assert_eq!(slide_index("ppt/slides/slide12.xml"), 12);
        assert_eq!(slide_index("ppt/slides/slide1.xml"), 1);
    }

    #[test]
    fn squeeze_collapses_blank_runs() {
        assert_eq!(squeeze_blank_lines("a\n\n\n\nb"), "a\n\nb");
    }

    #[test]
    fn trim_float_drops_trailing_zeros() {
        assert_eq!(trim_float(3.0), "3");
        assert_eq!(trim_float(3.5), "3.5");
    }
}
