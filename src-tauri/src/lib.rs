mod chat;
mod claude_md;
mod conv;
mod convert;
mod doctor;
mod kb;
mod provider;
mod skills;

use polaris_core::KbLocator;
use std::sync::Arc;
use tauri::Manager;

/// host 适配器：把板块② `kb` 的 `kb_root()` 适配成 core 的 [`KbLocator`] 契约，
/// 在启动时注入给板块⑤ `polaris-sandbox`，从而打破 `sandbox → kb` 的直接依赖。
/// （架构重构 Phase 1：依赖反转的落地点）
struct HostKbLocator;
impl KbLocator for HostKbLocator {
    fn kb_root(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(kb::kb_root())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let h = app.handle();
            kb::init(h).map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            // 注入 KbLocator 给 sandbox 板块 (须在 kb::init 之后, 命令执行之前)
            app.manage(Arc::new(HostKbLocator) as Arc<dyn KbLocator>);
            polaris_sandbox::init()
                .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            conv::init(h).map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            chat::init(h).map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            claude_md::init(h)
                .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            provider::init(h)
                .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // KB
            kb::kb_root,
            kb::kb_default_root,
            kb::kb_set_root,
            kb::kb_scan,
            kb::kb_list,
            kb::kb_read,
            kb::kb_delete,
            kb::kb_clear,
            kb::kb_search,
            kb::kb_ingest,
            kb::kb_upload_files,
            kb::kb_graph,
            // Sandbox (板块⑤ 已抽离为 polaris-sandbox crate, 命令名不变)
            polaris_sandbox::commands::sandbox_status,
            polaris_sandbox::commands::sandbox_build_image,
            polaris_sandbox::commands::sandbox_start,
            polaris_sandbox::commands::sandbox_stop,
            polaris_sandbox::commands::sandbox_exec,
            // CubeSandbox (E2B) 后端 — 「替换 Docker」可选后端
            polaris_sandbox::e2b::cube_config_get,
            polaris_sandbox::e2b::cube_config_set,
            polaris_sandbox::e2b::cube_status,
            // Conv (项目 + 对话历史)
            conv::conv_list_projects,
            conv::conv_create_project,
            conv::conv_archive_project,
            conv::conv_list_conversations,
            conv::conv_create_conversation,
            conv::conv_delete_conversation,
            conv::conv_rename_conversation,
            conv::conv_get_messages,
            // Chat
            chat::chat_send,
            chat::chat_cancel,
            chat::chat_attach_files,
            chat::artifact_read,
            chat::artifact_open_external,
            chat::artifact_reveal,
            chat::artifact_list,
            chat::artifact_search,
            // CLAUDE.md
            claude_md::claude_md_list_projects,
            claude_md::claude_md_kb_info,
            claude_md::claude_md_read,
            claude_md::claude_md_write,
            // Skills
            skills::list_skills,
            skills::get_skill,
            skills::create_skill,
            skills::install_skill,
            skills::import_skill,
            skills::delete_skill,
            // API 供应商坞 + 用量看板
            provider::provider_list,
            provider::provider_switch,
            provider::provider_save,
            provider::provider_delete,
            provider::usage_summary,
            provider::codex_status,
            provider::codex_login,
            // 环境医生 (环境监测 + 配置安装)
            doctor::env_check,
            doctor::env_fix_path,
            doctor::env_install_pi,
            doctor::env_install_node,
            doctor::env_install_pwsh,
            doctor::env_pi_update_check,
            doctor::env_update_pi,
            doctor::env_cancel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Polaris application");
}
