//! `os provider` — 供应商管理
//!
//! 支持列出、切换和验证供应商配置。

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct ProviderArgs {
    #[command(subcommand)]
    pub action: ProviderAction,
}

#[derive(Subcommand)]
pub enum ProviderAction {
    /// 列出供应商
    List {
        /// 应用类型 (claude/codex/gemini/opencode/openclaw/hermes)
        #[arg(long, default_value = "claude")]
        app: String,
        /// 仅显示当前活跃供应商
        #[arg(long)]
        current: bool,
    },
    /// 切换活跃供应商
    Switch {
        /// 供应商 ID
        #[arg(long)]
        id: Option<String>,
        /// 应用类型
        #[arg(long, default_value = "claude")]
        app: String,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
    /// 验证供应商 API Key
    Verify {
        /// API Base URL
        #[arg(long)]
        base_url: String,
        /// API Key
        #[arg(long)]
        api_key: String,
        /// 协议类型 (openai/anthropic)
        #[arg(long, default_value = "openai")]
        protocol: String,
    },
}

pub fn run(
    args: ProviderArgs,
    state: Option<&open_sunstar_lib::AppState>,
    json: bool,
) -> Result<(), String> {
    match args.action {
        ProviderAction::List { app, current } => {
            let state = state.ok_or_else(|| "数据库不可用".to_string())?;
            run_list(state, &app, current, json)
        }
        ProviderAction::Switch { id, app, yes } => {
            let state = state.ok_or_else(|| "数据库不可用".to_string())?;
            run_switch(state, id.as_deref(), &app, yes, json)
        }
        ProviderAction::Verify {
            base_url,
            api_key,
            protocol,
        } => run_verify(&base_url, &api_key, &protocol, json),
    }
}

fn run_list(
    state: &open_sunstar_lib::AppState,
    app: &str,
    current_only: bool,
    json: bool,
) -> Result<(), String> {
    let providers = state
        .db
        .get_all_providers(app)
        .map_err(|e| e.to_string())?;
    let current_id = state
        .db
        .get_current_provider(app)
        .map_err(|e| e.to_string())?;

    if current_only {
        if let Some(ref cid) = current_id {
            if let Some(p) = providers.get(cid) {
                if json {
                    let item = serde_json::json!({
                        "id": p.id,
                        "name": p.name,
                        "current": true,
                        "category": p.category,
                        "website_url": p.website_url,
                    });
                    output::print_result(&item, true);
                } else {
                    output::success(&format!(
                        "Current provider for {app}: {} ({})",
                        p.name, p.id
                    ));
                }
                return Ok(());
            }
        }
        if json {
            output::print_result(&serde_json::json!(null), true);
        } else {
            println!("No current provider set for {app}.");
        }
        return Ok(());
    }

    if json {
        let items: Vec<_> = providers
            .values()
            .map(|p| {
                let is_current = current_id.as_deref() == Some(&p.id);
                serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "current": is_current,
                    "category": p.category,
                    "website_url": p.website_url,
                })
            })
            .collect();
        output::print_result(&items, true);
    } else {
        if providers.is_empty() {
            println!("No providers configured for {app}.");
            return Ok(());
        }

        println!(
            "Providers for {app} ({} total):\n",
            providers.len()
        );
        println!(
            "  {:<6} {:<24} {:<12} {}",
            "ACTIVE", "NAME", "CATEGORY", "ID"
        );
        println!("  {}", "-".repeat(60));
        for p in providers.values() {
            let is_current = current_id.as_deref() == Some(&p.id);
            let active = if is_current { "✓" } else { "·" };
            let cat = p.category.as_deref().unwrap_or("-");
            println!(
                "  {:<6} {:<24} {:<12} {}",
                active, p.name, cat, p.id
            );
        }
    }

    Ok(())
}

fn run_switch(
    state: &open_sunstar_lib::AppState,
    id: Option<&str>,
    app: &str,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    // Resolve provider ID — interactive select if not provided
    let provider_id = match id {
        Some(id) => id.to_string(),
        None => {
            let providers =
                open_sunstar_lib::cli_api::cli_provider_list(state, app)?;
            let names: Vec<String> = providers.keys().cloned().collect();
            match output::select("Select provider", &names, json) {
                Some(idx) => names[idx].clone(),
                None => return Err("No provider selected".to_string()),
            }
        }
    };

    // 验证供应商存在
    let provider = state
        .db
        .get_provider_by_id(&provider_id, app)
        .map_err(|e| e.to_string())?;

    if provider.is_none() {
        return Err(format!(
            "供应商不存在: id={provider_id}, app={app}"
        ));
    }

    // Interactive confirmation (unless --yes or --json)
    output::header("Provider Switch");
    let current_id = state
        .db
        .get_current_provider(app)
        .map_err(|e| e.to_string())?;

    if let Some(ref cid) = current_id {
        output::info(&format!("Current:  {cid}"));
    } else {
        output::info("Current:  (none)");
    }
    let provider_ref = provider.as_ref().unwrap();
    output::info(&format!(
        "New:      {} ({})",
        provider_ref.name, provider_ref.id
    ));
    output::info(&format!("App:      {app}"));

    if !output::confirm("确认切换供应商?", yes || json, false) {
        output::info("已取消。");
        return Ok(());
    }

    let switch_result =
        open_sunstar_lib::cli_api::cli_provider_switch(state, app, &provider_id)?;

    if json {
        let result = serde_json::json!({
            "switched": true,
            "provider_id": provider_id,
            "app": app,
            "warnings": switch_result.warnings,
        });
        output::print_result(&result, true);
    } else {
        let name = provider_ref.name.clone();
        output::success(&format!(
            "Switched active provider for {app} to '{name}' ({provider_id})"
        ));
        for warning in &switch_result.warnings {
            output::warning(warning);
        }
    }

    Ok(())
}

fn run_verify(
    base_url: &str,
    api_key: &str,
    protocol: &str,
    json: bool,
) -> Result<(), String> {
    let proto = match protocol {
        "anthropic" => open_sunstar_lib::VerifyProtocol::Anthropic,
        _ => open_sunstar_lib::VerifyProtocol::OpenAi,
    };
    let result =
        open_sunstar_lib::cli_api::cli_provider_verify(base_url, api_key, proto)?;

    if json {
        output::print_result(&result, true);
    } else {
        if result.ok {
            output::success(&format!(
                "API Key verification succeeded. {} model(s) available.",
                result.model_count
            ));
        } else {
            let msg = result.error.as_deref().unwrap_or("Unknown error");
            output::error_msg(&format!("API Key verification failed: {msg}"));
        }
    }

    Ok(())
}
