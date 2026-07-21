pub(super) use super::{discovery::*, lifecycle::*, version::*};
use std::path::{Path, PathBuf};

#[test]
fn test_extract_version() {
    assert_eq!(extract_version("claude 1.0.20"), "1.0.20");
    assert_eq!(extract_version("v2.3.4-beta.1"), "2.3.4-beta.1");
    assert_eq!(extract_version("no version here"), "no version here");
}

#[test]
fn test_compare_semver() {
    use std::cmp::Ordering;
    assert_eq!(
        compare_semver("2.1.156", "2.1.154"),
        Some(Ordering::Greater)
    );
    assert_eq!(compare_semver("2.1.154", "2.1.156"), Some(Ordering::Less));
    assert_eq!(compare_semver("2.1.156", "2.1.156"), Some(Ordering::Equal));
    // 预发布 < 同核心正式版
    assert_eq!(
        compare_semver("2.1.156-beta.1", "2.1.156"),
        Some(Ordering::Less)
    );
    // core 更高的预发布仍高于较低的正式版（gemini nightly 场景）
    assert_eq!(
        compare_semver("0.45.0-nightly.1", "0.44.1"),
        Some(Ordering::Greater)
    );
    // 大 patch（codex 时间戳式）不溢出
    assert_eq!(
        compare_semver("0.1.2505172116", "0.135.0"),
        Some(Ordering::Less)
    );
    // 无法解析返回 None（gemini 的 `false` 脏 tag）
    assert_eq!(compare_semver("false", "1.0.0"), None);
}

#[test]
fn test_pick_latest_version() {
    use serde_json::json;
    let tags = json!({
        "latest": "2.1.154",
        "next": "2.1.156",
        "stable": "2.1.145"
    });
    let map = tags.as_object().unwrap();

    // 本地领先 latest（在 next 通道）→ 补查到 next，数字对齐
    assert_eq!(
        pick_latest_version(map, &["next"], Some("2.1.156")),
        Some("2.1.156".to_string())
    );
    // 本地等于 latest → 不补查，仍显示 latest
    assert_eq!(
        pick_latest_version(map, &["next"], Some("2.1.154")),
        Some("2.1.154".to_string())
    );
    // 本地落后 latest（稳定通道用户）→ 不补查，不被推向预发布版
    assert_eq!(
        pick_latest_version(map, &["next"], Some("2.1.145")),
        Some("2.1.154".to_string())
    );
    // 无预发布白名单 → 永远只看 latest（不解析 local，避免脏 local 触发）
    assert_eq!(
        pick_latest_version(map, &[], Some("2.1.156")),
        Some("2.1.154".to_string())
    );
    // 本地版本未知 → 保守只看 latest
    assert_eq!(
        pick_latest_version(map, &["next"], None),
        Some("2.1.154".to_string())
    );
}

#[test]
fn test_pick_latest_version_filters_dirty_prerelease() {
    use serde_json::json;
    // 模拟 codex：beta 是低于 latest 的时间戳式脏版本
    let tags = json!({
        "latest": "0.135.0",
        "beta": "0.1.2505172116"
    });
    let map = tags.as_object().unwrap();
    // 即便本地领先 latest，低于 latest 的脏 beta 也不会被选
    assert_eq!(
        pick_latest_version(map, &["beta"], Some("0.200.0")),
        Some("0.135.0".to_string())
    );
}

/// `parent_dir` 是锚定层"由 bin 路径推导同目录绝对路径"的基石,跨平台共用——
/// 这里固化 `\`/`/`/混合分隔符/根边界四种情况,避免未来重构悄悄改语义。
mod parent_dir_cases {
    use super::*;

    #[test]
    fn unix_path() {
        assert_eq!(
            parent_dir("/Users/me/.volta/bin/codex"),
            "/Users/me/.volta/bin"
        );
    }

    #[test]
    fn windows_backslash() {
        assert_eq!(
            parent_dir("C:\\Users\\me\\AppData\\Local\\Volta\\bin\\codex.exe"),
            "C:\\Users\\me\\AppData\\Local\\Volta\\bin"
        );
    }

    #[test]
    fn mixed_separators_takes_rightmost() {
        // Windows 上 `Path::join` 与字符串拼接可能产出混合分隔符;取**两种之中最右
        // 出现**的位置,而非"优先 `\`"——后者在混合时会取错父目录。
        assert_eq!(
            parent_dir("C:\\Users\\me/Code/openclaw\\codex.cmd"),
            "C:\\Users\\me/Code/openclaw"
        );
    }

    #[test]
    fn no_separator_returns_empty() {
        // 无父目录 → 空串,锚定层据此返 None、回退静态命令。
        assert_eq!(parent_dir("codex"), "");
    }

    #[test]
    fn separator_at_root_returns_empty() {
        // `/codex`:根目录是 index 0,`i > 0` 不满足 → 空串。同款行为对 Windows
        // 上的 `\codex` 也成立(实际不会出现,但语义对齐)。
        assert_eq!(parent_dir("/codex"), "");
        assert_eq!(parent_dir("\\codex"), "");
    }
}

/// Windows-only 锚定升级回归(等价类压缩到 3 种 idiom:volta/pnpm/npm)。整块通过
/// `cfg(target_os = "windows")` gate,在 macOS/Linux 上不参与 cargo test;Windows
/// CI 跑全套验证。tempdir 模拟 sibling 入口存在/不存在,锁定"扩展名顺序优先级 +
/// 含空格路径自动加双引号 + 探不到 sibling → None 退静态"三件事。
#[cfg(target_os = "windows")]
mod anchored_upgrade_windows {
    use super::*;

    /// 在 tempdir 下创建子目录 `subdir`(空字符串则用 tempdir 根),放入 `entry`
    /// 与若干 `siblings` 假文件。返回 `(TempDir, 子目录, 入口绝对路径)`——TempDir
    /// 必须保活,否则析构后 fs 文件消失、`is_file()` 失败,测试假绿。
    fn setup_sibling(
        subdir: &str,
        entry: &str,
        siblings: &[&str],
    ) -> (tempfile::TempDir, std::path::PathBuf, String) {
        let dir = tempfile::tempdir().unwrap();
        let sub = if subdir.is_empty() {
            dir.path().to_path_buf()
        } else {
            dir.path().join(subdir)
        };
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join(entry), "").unwrap();
        for s in siblings {
            std::fs::write(sub.join(s), "").unwrap();
        }
        let bin_path = sub.join(entry).to_string_lossy().to_string();
        (dir, sub, bin_path)
    }

    /// **必须与 `win_quote_path_for_batch` 主体保持镜像**——给 anchored 测试动态算
    /// expected,让用例在 temp 根目录含空格 / `&` / `(` / `%` 等特殊字符的开发机上
    /// 也能通过(默认 Windows `%TEMP%` = `C:\Users\<user>\AppData\Local\Temp`,
    /// 用户名带空格的机器整条 path 含空格、生产代码会正确加引号、测试硬编码无引号
    /// expected 会假失败)。
    ///
    /// 镜像引入"两边必须同步"的隐性依赖——回归防护层是 `win_quote_*` 那 7 个独立
    /// 单测,它们用硬编码字面值锁住 quoting 规则本身,即便此镜像漂移也会被那一组
    /// 测试 catch;反之亦然。
    fn expect_quoted_path(p: &str) -> String {
        let escaped = p.replace('%', "%%%%");
        let needs_quote = p
            .chars()
            .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
        if needs_quote {
            format!("\"{escaped}\"")
        } else {
            escaped
        }
    }

    #[test]
    fn volta_windows_uses_volta_install() {
        // tempdir 路径里不含 "volta" 子串,所以在 tempdir 下手建一个 `Volta` 子目录
        // 才能让 `infer_install_source` 通过路径 normalize 后命中 `/volta/` 分支。
        // sibling 候选顺序 `[exe, cmd]`——Volta 是 Rust 写的 native binary,首选 .exe。
        // expected 通过 `expect_quoted_path` 算出,以适应 temp 根目录含特殊字符的环境。
        let (_dir, sub, bin_path) = setup_sibling("Volta", "codex.cmd", &["volta.exe"]);
        let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
        let volta_full = format!("{}\\volta.exe", sub.to_string_lossy());
        let expected = format!(
            "{} update || call {} install @openai/codex",
            expect_quoted_path(&bin_path),
            expect_quoted_path(&volta_full)
        );
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn pnpm_windows_uses_pnpm_add() {
        // bin_path 落 `%LOCALAPPDATA%\pnpm\codex.cmd`,sibling 有 `pnpm.cmd` → 锚定到
        // `<dir>\pnpm.cmd add -g @openai/codex@latest`。用 add+@latest 而非 update,
        // 兼容"之前没通过 pnpm 装过"的幂等性场景。
        let (_dir, sub, bin_path) = setup_sibling("pnpm", "codex.cmd", &["pnpm.cmd"]);
        let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
        let pnpm_full = format!("{}\\pnpm.cmd", sub.to_string_lossy());
        let expected = format!(
            "{} update || call {} add -g @openai/codex@latest",
            expect_quoted_path(&bin_path),
            expect_quoted_path(&pnpm_full)
        );
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn opencode_windows_uses_package_fallback_without_official_upgrade() {
        let (_dir, sub, bin_path) = setup_sibling("pnpm", "opencode.cmd", &["pnpm.cmd"]);
        let cmd = anchored_command_from_paths("opencode", &bin_path, &bin_path);
        let pnpm_full = format!("{}\\pnpm.cmd", sub.to_string_lossy());
        let expected = format!(
            "{} add -g opencode-ai@latest",
            expect_quoted_path(&pnpm_full)
        );
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn opencode_windows_static_fallback_skips_official_upgrade() {
        let cmd = static_fallback_command("opencode");
        assert_eq!(cmd, "npm i -g opencode-ai@latest");
        assert!(!cmd.contains("opencode upgrade"));
    }

    #[test]
    fn npm_windows_default_branch() {
        // 任意 system 类路径(不命中 volta/pnpm)→ 兜底 sibling npm.cmd 锚定。
        // 模拟 nvm-windows 的实际形态:`<NVM_HOME>\v22.0.0\codex.cmd`。
        let (_dir, sub, bin_path) = setup_sibling("v22.0.0", "codex.cmd", &["npm.cmd"]);
        let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
        let npm_full = format!("{}\\npm.cmd", sub.to_string_lossy());
        let expected = format!(
            "{} update || call {} i -g @openai/codex@latest",
            expect_quoted_path(&bin_path),
            expect_quoted_path(&npm_full)
        );
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn windows_no_sibling_uses_cli_update_without_package_fallback() {
        // sibling npm.cmd 不存在(纯独立二进制)时,仍可锚定到 CLI 自身跑官方 update。
        // 只是没有包管理器 fallback。
        let (_dir, _sub, bin_path) = setup_sibling("", "codex.cmd", &[]);
        let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
        let expected = format!("{} update", expect_quoted_path(&bin_path));
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn hermes_windows_uses_cli_update() {
        // Hermes 自带 `hermes update`,不要再回退到 py/python/pip。即便同目录有
        // npm.cmd,也不应走 npm 分支。
        let (_dir, _sub, bin_path) = setup_sibling("", "hermes.exe", &["npm.cmd"]);
        let cmd = anchored_command_from_paths("hermes", &bin_path, &bin_path);
        let expected = format!("{} update", expect_quoted_path(&bin_path));
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn hermes_windows_static_fallback_uses_powershell_installer_without_pip() {
        let install = static_fallback_command_for("hermes", ToolLifecycleAction::Install);
        assert!(
            install.starts_with("powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand "),
            "should use PowerShell EncodedCommand installer: {install}"
        );
        let encoded = install
            .split_once("-EncodedCommand ")
            .map(|(_, encoded)| encoded)
            .expect("installer should include encoded command");
        assert_eq!(
            encoded,
            powershell_encoded_command(HERMES_INSTALL_WINDOWS_SCRIPT)
        );
        let install_prefix = install
            .split_once("-EncodedCommand ")
            .map(|(prefix, _)| prefix)
            .expect("installer should include encoded command");
        assert!(
            !install_prefix.contains("|")
                && !install_prefix.contains("-Command")
                && !install_prefix.contains("python")
                && !install_prefix.contains("pip"),
            "should hide PowerShell pipe from cmd.exe and avoid system Python/pip: {install}"
        );

        let update = static_fallback_command("hermes");
        assert!(
            update.starts_with(
                "hermes update || powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand "
            ),
            "should try CLI update before PowerShell installer: {update}"
        );
        let fallback = update
            .split_once("||")
            .map(|(_, fallback)| fallback)
            .expect("update should include a fallback command");
        let fallback_prefix = fallback
            .split_once("-EncodedCommand ")
            .map(|(prefix, _)| prefix)
            .expect("fallback should include encoded command");
        assert!(
                !fallback_prefix.contains('|')
                    && !fallback_prefix.contains("-Command")
                    && !update.contains("call powershell")
                    && !fallback_prefix.contains("python")
                    && !fallback_prefix.contains("pip"),
                "PowerShell fallback should be encoded, not called like a batch file or use pip: {update}"
            );
    }

    #[test]
    fn windows_path_with_space_is_double_quoted() {
        // 含空格的路径(`C:\Program Files\...`)在生成命令时必须用双引号包,否则
        // bat / cmd /C 解析会把第一个空格当 token 分隔符,后续参数串错。**精确等值断言
        // 锁定引号位置**(starts_with+contains 会放过"双引号位置错了但仍能命中"的回归)。
        let (_dir, sub, bin_path) = setup_sibling("Program Files", "codex.cmd", &["npm.cmd"]);
        let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
        let npm_full = format!("{}\\npm.cmd", sub.to_string_lossy());
        let expected = format!(
            "{} update || call {} i -g @openai/codex@latest",
            expect_quoted_path(&bin_path),
            expect_quoted_path(&npm_full)
        );
        assert_eq!(cmd.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn windows_full_batch_line_for_percent_path_uses_quadruple_escape() {
        // **完整生成的 batch 行**(`call ` + anchored cmd)对含字面 `%` 的路径必须
        // 4 倍转义 `%foo%` → `%%%%foo%%%%`:.bat parser 一轮还原为 `%%foo%%`,call
        // 二轮再还原为 `%foo%` 字面。helper 单测验证的是 `win_quote_path_for_batch`
        // 内部转义,这条 integration 测验证 anchored_command_from_paths 输出 + call
        // 包装后,**最终落到 .bat 的字符串**仍然闭合两轮 expansion。
        let (_dir, sub, bin_path) = setup_sibling("path%foo%", "codex.cmd", &["npm.cmd"]);
        let anchored = anchored_command_from_paths("codex", &bin_path, &bin_path).unwrap();
        // build_tool_action_line Windows 分支最终拼的就是 `call <anchored>`(中间
        // 没有其他变换),这里直接用 format! 复刻那一步,无需暴露内部 API。
        let batch_line = format!("call {anchored}");
        // 用 `expect_quoted_path` 算 npm 全路径的期望 quoting,**同时覆盖 temp 根
        // 含空格的环境**(否则 sub 本身含空格 + 子目录 `path%foo%` 触发 4 倍 `%` 转义
        // 会让 expected 漏引号、假失败)。
        let npm_full = format!("{}\\npm.cmd", sub.to_string_lossy());
        let expected = format!(
            "call {} update || call {} i -g @openai/codex@latest",
            expect_quoted_path(&bin_path),
            expect_quoted_path(&npm_full)
        );
        assert_eq!(batch_line, expected);
        // 双重锁定:确认 4 倍转义子串存在 + 不出现"残留的二倍转义或字面 `%foo%`"。
        assert!(
            batch_line.contains("%%%%foo%%%%"),
            "batch 行应含 4 倍转义 `%%%%foo%%%%`: {batch_line}"
        );
        assert!(
            !batch_line.contains("path%foo%"),
            "batch 行不应含未转义的字面 `%foo%`(会被 call 二次解析展开): {batch_line}"
        );
    }
}

/// Windows-only helpers 单测——在 macOS/Linux 上整块通过 cfg 排除,不参与 `cargo test`。
/// Windows CI(或本机 Windows 跑 cargo test)会激活这些用例。覆盖:①双引号
/// quoting 镜像 POSIX 版;②sibling_bin_with_ext 在 fs 上按 ext 顺序探到第一个存在的、
/// 全部不存在/空 dir 时返 None。tempdir 提供干净 fs 沙盒。
#[cfg(target_os = "windows")]
mod windows_helpers {
    use super::*;

    #[test]
    fn win_quote_clean_path_stays_bare() {
        // 普通路径不含特殊字符 → 不加引号,命令展示干净。
        assert_eq!(
            win_quote_path_for_batch("C:\\Users\\me\\npm.cmd"),
            "C:\\Users\\me\\npm.cmd"
        );
    }

    #[test]
    fn win_quote_spaced_path_gets_quoted() {
        assert_eq!(
            win_quote_path_for_batch("C:\\Program Files\\nodejs\\npm.cmd"),
            "\"C:\\Program Files\\nodejs\\npm.cmd\""
        );
    }

    #[test]
    fn win_quote_ampersand_path_gets_quoted() {
        // `&` 是 cmd 命令分隔符,NTFS 允许在路径中出现;没有引号会让 `call C:\A&B\npm.cmd`
        // 被解析为 `call C:\A` + `B\npm.cmd` 两条命令,执行错乱。
        assert_eq!(
            win_quote_path_for_batch("C:\\Tools&Dev\\npm.cmd"),
            "\"C:\\Tools&Dev\\npm.cmd\""
        );
    }

    #[test]
    fn win_quote_parens_path_gets_quoted() {
        // `(` / `)` 在 .bat 中是代码块语义,引号内才是字面意义。
        assert_eq!(
            win_quote_path_for_batch("C:\\Foo(x86)\\npm.cmd"),
            "\"C:\\Foo(x86)\\npm.cmd\""
        );
    }

    #[test]
    fn win_quote_caret_path_gets_quoted() {
        // `^` 是 cmd 的 escape character;包引号后是字面意义。
        assert_eq!(
            win_quote_path_for_batch("C:\\foo^bar\\npm.cmd"),
            "\"C:\\foo^bar\\npm.cmd\""
        );
    }

    #[test]
    fn win_quote_percent_is_escaped_to_quadruple_percent() {
        // `%` 经历 .bat 一轮 + call 二轮 expansion,要让 call 最终看到字面 `%FOO%`
        // 需要源 .bat 里写 `%%%%FOO%%%%`(一轮 → `%%FOO%%`,二轮 → `%FOO%` 字面)。
        // 用 `%%` 二倍转义只在 echo / 直接执行场景对,call 调用时会被还原成 variable
        // reference 进而被替换。**这一条用例锁住"call 二次解析"必须被 4 倍转义闭合**。
        assert_eq!(
            win_quote_path_for_batch("C:\\path%foo%\\npm.cmd"),
            "C:\\path%%%%foo%%%%\\npm.cmd"
        );
    }

    #[test]
    fn win_quote_percent_with_space_gets_both() {
        // `%` 4 倍转义与外层引号正交——含空格触发引号、含 `%` 触发 `%%%%` 转义,叠加。
        assert_eq!(
            win_quote_path_for_batch("C:\\my %dir%\\npm.cmd"),
            "\"C:\\my %%%%dir%%%%\\npm.cmd\""
        );
    }

    #[test]
    fn win_quote_needs_quote_uses_original_path() {
        // 回归 guard:`needs_quote` 判定基于**原路径**,不能用 escape 后字符串——
        // 否则原本无 token 边界字符的路径(如 `C:\path%foo%\npm.cmd`)在 escape
        // 引入更多 `%` 后被错误识别成"需要引号"。这是实现 bug 的隐性入口。
        // 入参不含任何 token 边界字符 → 不应加外层引号、只做 `%` 4 倍转义。
        let out = win_quote_path_for_batch("C:\\foo%bar%\\npm.cmd");
        assert!(!out.starts_with('"'), "纯 `%` 路径不应加外层引号: {out}");
    }

    #[test]
    fn sibling_bin_picks_first_existing_extension() {
        // 同目录同时存在 `npm.cmd` 和 `npm.exe` 时,候选顺序 `[cmd, exe]` 应取 .cmd——
        // 这是 Node.js 官方 installer 装出来的 idiom(.cmd 是入口、.exe 是 wrapper)。
        let dir = tempfile::tempdir().unwrap();
        let cmd_path = dir.path().join("npm.cmd");
        let exe_path = dir.path().join("npm.exe");
        std::fs::write(&cmd_path, "").unwrap();
        std::fs::write(&exe_path, "").unwrap();

        let codex = dir.path().join("codex.cmd").to_string_lossy().to_string();
        let found = sibling_bin_with_ext(&codex, "npm", &["cmd", "exe"]).unwrap();
        assert_eq!(found, cmd_path.to_string_lossy());
    }

    #[test]
    fn sibling_bin_volta_prefers_exe() {
        // Volta 是 Rust 写的 native binary,扩展名顺序应是 [exe, cmd]——若只有 .exe
        // 存在(常见情形),探到的就是它。
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("volta.exe");
        std::fs::write(&exe_path, "").unwrap();

        let codex = dir.path().join("codex.exe").to_string_lossy().to_string();
        let found = sibling_bin_with_ext(&codex, "volta", &["exe", "cmd"]).unwrap();
        assert_eq!(found, exe_path.to_string_lossy());
    }

    #[test]
    fn sibling_bin_returns_none_when_none_exist() {
        // 同目录下没有任何候选 → None,锚定层据此退到静态命令。
        let dir = tempfile::tempdir().unwrap();
        let codex = dir.path().join("codex.cmd").to_string_lossy().to_string();
        assert!(sibling_bin_with_ext(&codex, "npm", &["cmd", "exe"]).is_none());
    }

    #[test]
    fn sibling_bin_returns_none_when_no_parent() {
        // bin_path 没有目录部分(纯文件名) → parent_dir 空串 → 返 None。
        assert!(sibling_bin_with_ext("codex.cmd", "npm", &["cmd"]).is_none());
    }

    #[test]
    fn wsl_hermes_command_uses_unix_installer_not_powershell_or_pip() {
        // 跨 wsl.exe 边界后跑的是 Linux,Windows PowerShell installer 不适用;
        // 也不要再走 python3/python pip 链,避免 Python 版本/pyenv shim 问题。
        let update_cmd =
            wsl_tool_action_shell_command("hermes", ToolLifecycleAction::Update).unwrap();
        assert!(
            update_cmd.starts_with("hermes update || bash -c 'tmp=$(mktemp) && curl -fsSL "),
            "WSL hermes 更新应先尝试 CLI 自更新再回退官方 installer,得到: {update_cmd}"
        );
        let fallback = update_cmd
            .split_once("||")
            .map(|(_, fallback)| fallback)
            .expect("update should include installer fallback");
        assert!(
            !fallback.contains('|')
                && fallback.contains(" -o $tmp && bash $tmp")
                && !update_cmd.contains("powershell")
                && !update_cmd.contains("pip"),
            "WSL hermes fallback 不能依赖 pipefail/Windows installer/pip,得到: {update_cmd}"
        );

        let install_cmd =
            wsl_tool_action_shell_command("hermes", ToolLifecycleAction::Install).unwrap();
        assert!(
            install_cmd.starts_with("bash -c 'tmp=$(mktemp) && curl -fsSL "),
            "WSL hermes 安装应直接走官方 Unix installer,得到: {install_cmd}"
        );
        assert!(
            !install_cmd.contains('|') && install_cmd.contains(" -o $tmp && bash $tmp"),
            "WSL hermes 安装不应依赖 pipefail,得到: {install_cmd}"
        );
    }

    #[test]
    fn wsl_hermes_install_line_does_not_depend_on_outer_pipefail() {
        let line = build_wsl_tool_action_line("Ubuntu", HERMES_INSTALL_UNIX, None, None)
            .expect("valid WSL command line");
        assert!(line.starts_with("wsl.exe -d Ubuntu -- sh -c "));
        assert!(
            !line.contains("| bash") && line.contains(" -o $tmp && bash $tmp"),
            "WSL 子 shell 内不能出现 curl 管道安装器: {line}"
        );
    }

    #[test]
    fn wsl_install_uses_posix_install_priority() {
        let claude = wsl_tool_action_shell_command("claude", ToolLifecycleAction::Install).unwrap();
        assert!(
            claude
                .starts_with("bash -c 'tmp=$(mktemp) && curl -fsSL https://claude.ai/install.sh ")
                && claude.contains(" || npm i -g @anthropic-ai/claude-code@latest"),
            "WSL claude install should prefer native POSIX installer with npm fallback: {claude}"
        );
        assert!(!claude.contains("| bash"));

        let opencode =
            wsl_tool_action_shell_command("opencode", ToolLifecycleAction::Install).unwrap();
        assert!(
                opencode.starts_with(
                    "bash -c 'tmp=$(mktemp) && curl -fsSL https://opencode.ai/install "
                ) && opencode.contains(" || npm i -g opencode-ai@latest"),
                "WSL opencode install should prefer native POSIX installer with npm fallback: {opencode}"
            );
        assert!(!opencode.contains("| bash"));

        let codex = wsl_tool_action_shell_command("codex", ToolLifecycleAction::Install).unwrap();
        assert_eq!(codex, "npm i -g @openai/codex@latest");
    }

    #[test]
    fn wsl_npm_tools_use_posix_update_chain_without_batch_call() {
        // WSL 内跑的是 POSIX shell,不能带 Windows batch 的 `call`。同时 update
        // fallback 仍应先尝试官方 CLI 自升级。
        let cmd = wsl_tool_action_shell_command("claude", ToolLifecycleAction::Update).unwrap();
        assert_eq!(
            cmd,
            "claude update || npm i -g @anthropic-ai/claude-code@latest"
        );
    }
}

/// `infer_install_source` 是判定锚定 idiom 的入口——nvm/homebrew/volta/pnpm/...
/// 各对应不同的升级命令形态。函数内部已 `replace('\\','/').to_ascii_lowercase()`
/// 归一化,Windows 反斜杠 + 大小写差异在此处不需要分平台。这里固化"哪条路径
/// 算哪种来源"的归类断言,避免未来调整子串顺序时静默改变分类。
mod install_source_classification {
    use super::*;
    use std::path::Path;

    #[test]
    fn macos_volta_with_dot_prefix() {
        assert_eq!(
            infer_install_source(Path::new("/Users/me/.volta/bin/codex")),
            "volta"
        );
    }

    #[test]
    fn windows_volta_localappdata_no_dot() {
        // `%LOCALAPPDATA%\Volta\bin\codex.exe` —— 没有前导点,靠兜底的 `/volta/`
        // 命中(归一化后小写)。如果只识别 `/.volta/`,Windows 这一类会落到 system。
        assert_eq!(
            infer_install_source(Path::new(
                "C:\\Users\\me\\AppData\\Local\\Volta\\bin\\codex.exe"
            )),
            "volta"
        );
    }

    #[test]
    fn windows_pnpm_localappdata() {
        // `%LOCALAPPDATA%\pnpm\codex.cmd` —— pnpm 全局 bin 目录,识别为 pnpm 后
        // 锚定命令走 `pnpm add -g <pkg>@latest`,而不是 sibling npm。
        assert_eq!(
            infer_install_source(Path::new("C:\\Users\\me\\AppData\\Local\\pnpm\\codex.cmd")),
            "pnpm"
        );
    }

    #[test]
    fn windows_nvm_falls_back_to_system() {
        // nvm-windows 安装的工具路径不含 `.nvm`(它通常装在 `%APPDATA%\nvm` 或
        // `C:\Program Files\nodejs` symlink),刻意不识别成专属 source——锚定层
        // 会按 system → sibling npm.cmd 处理,跟 nvm-windows 的实际 idiom 一致
        // (它的全局包就是当前选中的 node 的 npm 装的)。
        assert_eq!(
            infer_install_source(Path::new(
                "C:\\Users\\me\\AppData\\Roaming\\nvm\\v22.0.0\\codex.cmd"
            )),
            "system"
        );
    }

    #[test]
    fn windows_scoop_still_identified() {
        // Scoop 已有 `/scoop/` 分支;我们的 6 个工具都不是 scoop formula,所以这条
        // 实际不影响锚定决策(锚定层会用 sibling npm.cmd),但归类保留方便未来。
        assert_eq!(
            infer_install_source(Path::new("C:\\Users\\me\\scoop\\shims\\codex.cmd")),
            "scoop"
        );
    }
}

/// 锚定升级命令生成：用真实勘察到的安装路径固化为回归断言——
/// 一台机器上 4 个工具恰好对应 4 种升级方式（原生 self-update / brew / nvm npm /
/// homebrew npm），任何改动若打破其中一种都会立刻被这些用例拦下。
#[cfg(not(target_os = "windows"))]
mod anchored_upgrade {
    use super::*;
    use std::path::Path;

    fn inst(path: &str, is_default: bool) -> ToolInstallation {
        ToolInstallation {
            path: path.to_string(),
            version: None,
            runnable: true,
            error: None,
            source: infer_install_source(Path::new(path)).to_string(),
            is_path_default: is_default,
            // 测试场景下不需要走 fs canonicalize——POSIX 锚定测试关心的是
            // path/real 都被传给 anchored_command_from_paths 的纯字符串判定,
            // 已有用例(brew_formula_extraction / claude_native_*)是直接
            // 调 anchored_command_from_paths,不通过 installs_anchored_command,
            // 这里 real 是给上层 default_install + read 用,填同值即可。
            real: std::path::PathBuf::from(path),
        }
    }

    #[test]
    fn claude_native_installer_uses_self_update() {
        // ~/.local/bin/claude → 真身在 ~/.local/share/claude/versions/,自带 self-update;
        // 它不归 npm 管,且在 PATH 里比 nvm/homebrew 更靠前,用 npm 升级纯属白装。
        // **绝对路径调用 launcher** 避免 GUI 非登录 `bash -c` 时 PATH 没有
        // ~/.local/bin 导致 `claude: not found`(exit 127)而失败。
        let cmd = anchored_command_from_paths(
            "claude",
            "/Users/me/.local/bin/claude",
            "/Users/me/.local/share/claude/versions/2.1.146",
        );
        assert_eq!(cmd.as_deref(), Some("/Users/me/.local/bin/claude update"));
    }

    #[test]
    fn gemini_homebrew_formula_uses_brew_upgrade() {
        // /opt/homebrew/bin/gemini → Cellar/gemini-cli/...:是 brew formula 而非 npm 全局包,
        // 且 formula 名(gemini-cli) ≠ npm 包名(@google/gemini-cli)。
        // **brew 与 formula 入口同目录**,用 `<dir>/brew` 绝对路径调用,避免 GUI
        // 非登录 `bash -c` 时 PATH 没有 /opt/homebrew/bin 导致 `brew: not found`。
        let cmd = anchored_command_from_paths(
                "gemini",
                "/opt/homebrew/bin/gemini",
                "/opt/homebrew/Cellar/gemini-cli/0.13.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js",
            );
        assert_eq!(
            cmd.as_deref(),
            Some("/opt/homebrew/bin/brew upgrade gemini-cli")
        );
    }

    #[test]
    fn codex_homebrew_formula_uses_brew_not_self_update() {
        // Homebrew formula 归 brew 管理;即使 Codex 有 self-update,也不先改动
        // Cellar 内的安装内容。
        let cmd = anchored_command_from_paths(
            "codex",
            "/opt/homebrew/bin/codex",
            "/opt/homebrew/Cellar/codex/1.2.3/bin/codex",
        );
        assert_eq!(cmd.as_deref(), Some("/opt/homebrew/bin/brew upgrade codex"));
    }

    #[test]
    fn gemini_nvm_anchors_to_npm_without_cli_update() {
        let cmd = anchored_command_from_paths(
                "gemini",
                "/Users/me/.nvm/versions/node/v22.14.0/bin/gemini",
                "/Users/me/.nvm/versions/node/v22.14.0/lib/node_modules/@google/gemini-cli/dist/index.js",
            );
        assert_eq!(
            cmd.as_deref(),
            Some("/Users/me/.nvm/versions/node/v22.14.0/bin/npm i -g @google/gemini-cli@latest")
        );
    }

    #[test]
    fn codex_nvm_anchors_to_that_npm() {
        // Codex 官方 self-update 只在支持的 release 上生效;失败时仍写回同一个
        // node 的 npm，而非 PATH 第一个 npm。
        let cmd = anchored_command_from_paths(
            "codex",
            "/Users/me/.nvm/versions/node/v22.14.0/bin/codex",
            "/Users/me/.nvm/versions/node/v22.14.0/lib/node_modules/@openai/codex/bin/codex.js",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("/Users/me/.nvm/versions/node/v22.14.0/bin/codex update || /Users/me/.nvm/versions/node/v22.14.0/bin/npm i -g @openai/codex@latest")
            );
    }

    #[test]
    fn homebrew_npm_global_package_anchors_not_brew() {
        // openclaw 装在 Homebrew node 的全局目录(lib/node_modules，非 Cellar)：
        // 是 npm 全局包，官方 update 失败后走 npm 锚定而非 brew upgrade。
        let cmd = anchored_command_from_paths(
            "openclaw",
            "/opt/homebrew/bin/openclaw",
            "/opt/homebrew/lib/node_modules/openclaw/openclaw.mjs",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("/opt/homebrew/bin/openclaw update --yes || /opt/homebrew/bin/npm i -g openclaw@latest")
            );
    }

    #[test]
    fn volta_uses_volta_install() {
        // `~/.volta/bin` 通常不在 GUI 非登录 `bash -c` 的 PATH 里,且用户可能
        // PATH 上还有另一份 volta → 必须绝对路径锚定到命令行命中的这一份。
        let cmd = anchored_command_from_paths(
            "codex",
            "/Users/me/.volta/bin/codex",
            "/Users/me/.volta/tools/image/packages/codex/lib/node_modules/@openai/codex",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("/Users/me/.volta/bin/codex update || /Users/me/.volta/bin/volta install @openai/codex")
            );
    }

    #[test]
    fn bun_uses_bun_add() {
        // OpenCode 先跑官方 upgrade;失败后 bun 同 volta:绝对路径写回原安装源。
        let cmd = anchored_command_from_paths(
            "opencode",
            "/Users/me/.bun/bin/opencode",
            "/Users/me/.bun/install/global/node_modules/opencode-ai/bin/opencode",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("/Users/me/.bun/bin/opencode upgrade || /Users/me/.bun/bin/bun add -g opencode-ai@latest")
            );
    }

    #[test]
    fn volta_path_with_space_is_quoted() {
        // volta 分支用 `<dir>/volta`,目录含空格时同样要 POSIX 引号包裹。
        let cmd = anchored_command_from_paths(
            "codex",
            "/Users/my name/.volta/bin/codex",
            "/Users/my name/.volta/tools/image/packages/codex/lib/node_modules/@openai/codex",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.volta/bin/codex' update || '/Users/my name/.volta/bin/volta' install @openai/codex")
            );
    }

    #[test]
    fn bun_path_with_space_is_quoted() {
        // bun 分支与 volta 共享 sibling_bin + quote_path_if_spaced,
        // 这条用例锁住 `bun add -g` 命令头部的引号包裹形态。
        let cmd = anchored_command_from_paths(
            "opencode",
            "/Users/my name/.bun/bin/opencode",
            "/Users/my name/.bun/install/global/node_modules/opencode-ai/bin/opencode",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.bun/bin/opencode' upgrade || '/Users/my name/.bun/bin/bun' add -g opencode-ai@latest")
            );
    }

    #[test]
    fn hermes_uses_cli_update_anchor() {
        // Hermes 自带 `hermes update`;锚定到命令行默认那处 CLI,避免 OpenSunstar 猜
        // 系统 Python/pip 时撞上 Python >=3.11 或 pyenv shim 问题。
        let cmd =
            anchored_command_from_paths("hermes", "/usr/local/bin/hermes", "/usr/local/bin/hermes");
        assert_eq!(cmd.as_deref(), Some("/usr/local/bin/hermes update"));
    }

    #[test]
    fn opencode_native_install_uses_cli_upgrade_without_package_fallback() {
        // opencode install.sh 装到 ~/.opencode/bin（独立二进制、无同级 npm）：
        // 不能锚定到 `<dir>/npm`（必失败），但可以锚定到 CLI 自身跑官方 upgrade。
        let cmd = anchored_command_from_paths(
            "opencode",
            "/Users/me/.opencode/bin/opencode",
            "/Users/me/.opencode/bin/opencode",
        );
        assert_eq!(
            cmd.as_deref(),
            Some("/Users/me/.opencode/bin/opencode upgrade")
        );
    }

    #[test]
    fn go_bin_opencode_uses_cli_upgrade_without_package_fallback() {
        // ~/go/bin 同理：无同级 npm，但 OpenCode 官方 upgrade 可由 CLI 自己处理。
        let cmd = anchored_command_from_paths(
            "opencode",
            "/Users/me/go/bin/opencode",
            "/Users/me/go/bin/opencode",
        );
        assert_eq!(cmd.as_deref(), Some("/Users/me/go/bin/opencode upgrade"));
    }

    #[test]
    fn fnm_install_anchors_to_that_npm() {
        // fnm 是自带同级 npm 的 node 管理器 → 锚定到那处的 npm。
        let cmd = anchored_command_from_paths(
                "codex",
                "/Users/me/.local/share/fnm_multishells/12345_abc/bin/codex",
                "/Users/me/.local/share/fnm_multishells/12345_abc/lib/node_modules/@openai/codex/bin/codex.js",
            );
        assert_eq!(
                cmd.as_deref(),
                Some(
                    "/Users/me/.local/share/fnm_multishells/12345_abc/bin/codex update || /Users/me/.local/share/fnm_multishells/12345_abc/bin/npm i -g @openai/codex@latest"
                )
            );
    }

    #[test]
    fn path_with_space_is_quoted() {
        let cmd = anchored_command_from_paths(
            "codex",
            "/Users/my name/.nvm/versions/node/v22/bin/codex",
            "/Users/my name/.nvm/versions/node/v22/lib/node_modules/@openai/codex/bin/codex.js",
        );
        assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.nvm/versions/node/v22/bin/codex' update || '/Users/my name/.nvm/versions/node/v22/bin/npm' i -g @openai/codex@latest")
            );
    }

    #[test]
    fn claude_native_path_with_space_is_quoted() {
        // claude 分支同样要 POSIX 引号包裹含空格的 bin_path,
        // 否则 `/Users/my name/.local/bin/claude update` 会被 shell 拆词。
        let cmd = anchored_command_from_paths(
            "claude",
            "/Users/my name/.local/bin/claude",
            "/Users/my name/.local/share/claude/versions/2.1.146",
        );
        assert_eq!(
            cmd.as_deref(),
            Some("'/Users/my name/.local/bin/claude' update")
        );
    }

    #[test]
    fn brew_path_with_space_is_quoted() {
        // brew 分支用 `<bin_path 同目录>/brew`,目录含空格时同样要引号包裹。
        let cmd = anchored_command_from_paths(
                "gemini",
                "/opt/my brew/bin/gemini",
                "/opt/my brew/Cellar/gemini-cli/0.13.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js",
            );
        assert_eq!(
            cmd.as_deref(),
            Some("'/opt/my brew/bin/brew' upgrade gemini-cli")
        );
    }

    #[test]
    fn brew_formula_extraction() {
        assert_eq!(
            brew_formula_from_path("/opt/homebrew/Cellar/gemini-cli/0.13.0/bin/gemini").as_deref(),
            Some("gemini-cli")
        );
        // node 全局包不在 Cellar 下 → 不是 formula。
        assert_eq!(
            brew_formula_from_path("/opt/homebrew/lib/node_modules/openclaw/openclaw.mjs"),
            None
        );
        assert_eq!(
            brew_formula_from_path("/Users/me/.nvm/versions/node/v22/lib/node_modules/x"),
            None
        );
    }

    #[test]
    fn sibling_bin_returns_none_when_bin_path_has_no_directory() {
        // bin_path 不含 `/` → parent_dir 返回空 → sibling_bin 不能拼出绝对路径
        // → None,让上游 anchored_command_from_paths 整体退化为静态命令兜底,
        // 而不是悄悄拼出 `npm i -g <pkg>` 这种依赖 PATH 的指令(违背"必须绝对路径"
        // 不变量)。实际从 enumerate_tool_installations 走的 bin_path 都是绝对路径,
        // 这条防线不期望被触发,但闭合了 helper 与函数文档的语义一致。
        assert_eq!(sibling_bin("codex", "npm"), None);
        assert_eq!(sibling_bin("", "brew"), None);
        // 含 `/` 即可拼出绝对路径——这是常规路径。
        assert_eq!(
            sibling_bin("/opt/homebrew/bin/gemini", "brew").as_deref(),
            Some("/opt/homebrew/bin/brew")
        );
    }

    #[test]
    fn default_install_prefers_path_default() {
        let installs = vec![
            inst("/opt/homebrew/bin/openclaw", false),
            inst("/Users/me/.nvm/versions/node/v22/bin/openclaw", true),
        ];
        assert_eq!(
            default_install(&installs).map(|i| i.path.as_str()),
            Some("/Users/me/.nvm/versions/node/v22/bin/openclaw")
        );
    }

    #[test]
    fn default_install_falls_back_to_sole_entry() {
        let installs = vec![inst("/opt/homebrew/bin/gemini", false)];
        assert_eq!(
            default_install(&installs).map(|i| i.path.as_str()),
            Some("/opt/homebrew/bin/gemini")
        );
    }

    #[test]
    fn default_install_none_when_ambiguous() {
        let installs = vec![
            inst("/opt/homebrew/bin/openclaw", false),
            inst("/Users/me/.nvm/versions/node/v22/bin/openclaw", false),
        ];
        assert!(default_install(&installs).is_none());
    }

    #[test]
    fn first_abs_path_line_skips_shell_noise() {
        // 交互式 .zshrc 先打印欢迎语（如 powerlevel10k / 自定义提示），
        // command -v 的真实路径在其后 → 跳过噪音取真路径。
        assert_eq!(
            first_abs_path_line("🚀 Welcome back!\n/Users/me/.local/bin/claude\n"),
            Some("/Users/me/.local/bin/claude")
        );
        // 无噪音时取第一行。
        assert_eq!(
            first_abs_path_line("/opt/homebrew/bin/gemini\n"),
            Some("/opt/homebrew/bin/gemini")
        );
        // 输出里没有任何绝对路径 → None。
        assert_eq!(first_abs_path_line("welcome\nbye\n"), None);
    }

    #[test]
    fn is_conflicting_thresholds() {
        let make = |version: Option<&str>, runnable: bool| ToolInstallation {
            path: "/x".to_string(),
            version: version.map(str::to_string),
            runnable,
            error: None,
            source: "nvm".to_string(),
            is_path_default: false,
            real: std::path::PathBuf::from("/x"),
        };
        // 单处 → 不冲突。
        assert!(!is_conflicting(&[make(Some("1.0.0"), true)]));
        // 两处同版本、都能跑 → 不冲突（同版本装两遍不打扰）。
        assert!(!is_conflicting(&[
            make(Some("1.0.0"), true),
            make(Some("1.0.0"), true)
        ]));
        // 版本分歧 → 冲突。
        assert!(is_conflicting(&[
            make(Some("1.0.0"), true),
            make(Some("2.0.0"), true)
        ]));
        // 同版本但运行态混合（一个能跑、一个跑不起来）→ 冲突。
        assert!(is_conflicting(&[
            make(Some("1.0.0"), true),
            make(Some("1.0.0"), false)
        ]));
    }
}

/// install 端的"上游推荐 || npm 兜底"短路链:把工具→官方安装方式这一上游事实
/// 固化为回归断言。任何方案改动若打破短路链结构或 URL,都会被这些用例拦下。
#[cfg(not(target_os = "windows"))]
mod install_strategy {
    use super::*;

    #[test]
    fn claude_install_prefers_native_with_npm_fallback() {
        // Anthropic 现在主推 native installer(claude.ai/install.sh),
        // 网络不通时短路到 npm 仍能装上;两段都得在,顺序也得对。
        let cmd = install_command_for("claude");
        assert!(
            cmd.contains("https://claude.ai/install.sh"),
            "should include official installer URL: {cmd}"
        );
        assert!(
            cmd.contains("@anthropic-ai/claude-code@latest"),
            "should keep npm package as fallback: {cmd}"
        );
        let parts: Vec<&str> = cmd.split("||").collect();
        assert_eq!(parts.len(), 2, "should be a two-step short-circuit chain");
        assert!(parts[0].contains("install.sh"), "native first: {cmd}");
        assert!(
            !parts[0].contains('|'),
            "native installer should avoid pipe: {cmd}"
        );
        assert!(parts[1].contains("npm i -g"), "npm second: {cmd}");
    }

    #[test]
    fn opencode_install_prefers_native_with_npm_fallback() {
        // SST 自家 install.sh 与 claude 同形态:bash 脚本、网络下载、装到 ~/.opencode/bin。
        let cmd = install_command_for("opencode");
        assert!(
            cmd.contains("https://opencode.ai/install"),
            "should include official installer URL: {cmd}"
        );
        assert!(
            cmd.contains("opencode-ai@latest"),
            "should keep npm package as fallback: {cmd}"
        );
        assert!(cmd.contains("||"), "should chain fallback: {cmd}");
        assert!(
            !cmd.split("||").next().unwrap_or_default().contains('|'),
            "native installer should avoid pipe: {cmd}"
        );
    }

    #[test]
    fn codex_install_keeps_static_npm() {
        // OpenAI 暂无独立 native installer,保持原裸 npm,不引入兜底链(无东西可兜底)。
        let cmd = install_command_for("codex");
        assert_eq!(cmd, "npm i -g @openai/codex@latest");
        assert!(!cmd.contains("||"));
    }

    #[test]
    fn gemini_install_keeps_static_npm() {
        // Google 文档同时支持 brew/npm,但本表保持与 update fallback 一致的 npm。
        // 用户若已装 brew gemini-cli,update 路径的锚定会识别 formula → brew upgrade,
        // 所以 install 端不强行替用户决策"用 brew 还是 npm"。
        let cmd = install_command_for("gemini");
        assert_eq!(cmd, "npm i -g @google/gemini-cli@latest");
    }

    #[test]
    fn openclaw_install_keeps_static_npm() {
        let cmd = install_command_for("openclaw");
        assert_eq!(cmd, "npm i -g openclaw@latest");
    }

    #[test]
    fn update_fallbacks_use_official_cli_only_when_supported() {
        assert_eq!(
            static_fallback_command("claude"),
            "claude update || npm i -g @anthropic-ai/claude-code@latest"
        );
        assert_eq!(
            static_fallback_command("codex"),
            "codex update || npm i -g @openai/codex@latest"
        );
        assert_eq!(
            static_fallback_command("gemini"),
            "npm i -g @google/gemini-cli@latest"
        );
        assert!(!static_fallback_command("gemini").contains("gemini update"));
        assert_eq!(
            static_fallback_command("opencode"),
            "opencode upgrade || npm i -g opencode-ai@latest"
        );
        assert_eq!(
            static_fallback_command("openclaw"),
            "openclaw update --yes || npm i -g openclaw@latest"
        );
    }

    #[test]
    fn hermes_install_uses_official_installer() {
        // Hermes 官方 installer 会处理 Python 3.11+/uv 等运行时;不要再从 OpenSunstar
        // 里走 `python3 || python` pip 链。
        let cmd = install_command_for("hermes");
        assert!(
            cmd.starts_with("bash -c 'tmp=$(mktemp) && curl -fsSL ")
                && cmd.contains("install.sh -o $tmp && bash $tmp"),
            "should use official installer: {cmd}"
        );
        assert!(
            !cmd.contains('|') && !cmd.contains("python") && !cmd.contains("pip"),
            "should not depend on pipefail or system Python/pip: {cmd}"
        );
    }

    #[test]
    fn hermes_update_fallback_uses_cli_update_then_installer() {
        // 锚定失败时也不回退 pip:先让 PATH 上的 hermes 自更新,找不到/失败再跑官方
        // installer。这样 pyenv 的 `python` shim 不会参与错误路径。
        let cmd = static_fallback_command("hermes");
        assert!(
            cmd.starts_with("hermes update || bash -c 'tmp=$(mktemp) && curl -fsSL "),
            "should try CLI update before official installer: {cmd}"
        );
        let fallback = cmd
            .split_once("||")
            .map(|(_, fallback)| fallback)
            .expect("update should include installer fallback");
        assert!(
            !fallback.contains('|') && !cmd.contains("python") && !cmd.contains("pip"),
            "should not depend on pipefail or system Python/pip: {cmd}"
        );
    }
}

#[cfg(target_os = "windows")]
mod wsl_helpers {
    use super::*;

    #[test]
    fn test_is_valid_shell() {
        assert!(is_valid_shell("bash"));
        assert!(is_valid_shell("zsh"));
        assert!(is_valid_shell("sh"));
        assert!(is_valid_shell("fish"));
        assert!(is_valid_shell("dash"));
        assert!(is_valid_shell("/usr/bin/bash"));
        assert!(is_valid_shell("/bin/zsh"));
        assert!(!is_valid_shell("powershell"));
        assert!(!is_valid_shell("cmd"));
        assert!(!is_valid_shell(""));
    }

    #[test]
    fn test_is_valid_shell_flag() {
        assert!(is_valid_shell_flag("-c"));
        assert!(is_valid_shell_flag("-lc"));
        assert!(is_valid_shell_flag("-lic"));
        assert!(!is_valid_shell_flag("-x"));
        assert!(!is_valid_shell_flag(""));
        assert!(!is_valid_shell_flag("--login"));
    }

    #[test]
    fn test_default_flag_for_shell() {
        assert_eq!(default_flag_for_shell("sh"), "-c");
        assert_eq!(default_flag_for_shell("dash"), "-c");
        assert_eq!(default_flag_for_shell("/bin/dash"), "-c");
        assert_eq!(default_flag_for_shell("fish"), "-lc");
        assert_eq!(default_flag_for_shell("bash"), "-lic");
        assert_eq!(default_flag_for_shell("zsh"), "-lic");
        assert_eq!(default_flag_for_shell("/usr/bin/zsh"), "-lic");
    }

    #[test]
    fn test_is_valid_wsl_distro_name() {
        assert!(is_valid_wsl_distro_name("Ubuntu"));
        assert!(is_valid_wsl_distro_name("Ubuntu-22.04"));
        assert!(is_valid_wsl_distro_name("my_distro"));
        assert!(!is_valid_wsl_distro_name(""));
        assert!(!is_valid_wsl_distro_name("distro with spaces"));
        assert!(!is_valid_wsl_distro_name(&"a".repeat(65)));
    }
}

#[test]
fn opencode_extra_search_paths_includes_install_and_fallback_dirs() {
    let home = PathBuf::from("/home/tester");
    let install_dir = Some(std::ffi::OsString::from("/custom/opencode/bin"));
    let xdg_bin_dir = Some(std::ffi::OsString::from("/xdg/bin"));
    let gopath =
        std::env::join_paths([PathBuf::from("/go/path1"), PathBuf::from("/go/path2")]).ok();

    let paths = opencode_extra_search_paths(&home, install_dir, xdg_bin_dir, gopath);

    assert_eq!(paths[0], PathBuf::from("/custom/opencode/bin"));
    assert_eq!(paths[1], PathBuf::from("/xdg/bin"));
    assert!(paths.contains(&PathBuf::from("/home/tester/bin")));
    assert!(paths.contains(&PathBuf::from("/home/tester/.opencode/bin")));
    assert!(paths.contains(&PathBuf::from("/home/tester/.bun/bin")));
    assert!(paths.contains(&PathBuf::from("/home/tester/go/bin")));
    assert!(paths.contains(&PathBuf::from("/go/path1/bin")));
    assert!(paths.contains(&PathBuf::from("/go/path2/bin")));
}

#[test]
fn opencode_extra_search_paths_deduplicates_repeated_entries() {
    let home = PathBuf::from("/home/tester");
    let same_dir = Some(std::ffi::OsString::from("/same/path"));

    let paths = opencode_extra_search_paths(&home, same_dir.clone(), same_dir, None);

    let count = paths
        .iter()
        .filter(|path| path.as_path() == Path::new("/same/path"))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn opencode_extra_search_paths_deduplicates_bun_default_dir() {
    let home = PathBuf::from("/home/tester");
    let paths = opencode_extra_search_paths(&home, None, None, None);

    let count = paths
        .iter()
        .filter(|path| path.as_path() == Path::new("/home/tester/.bun/bin"))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn cli_path_env_search_paths_include_path_entries_and_dedupe() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    std::fs::create_dir_all(&first).expect("first dir should be created");
    std::fs::create_dir_all(&second).expect("second dir should be created");

    let path_env = std::env::join_paths([first.clone(), second.clone(), first.clone()])
        .expect("test path env should be joinable");
    let mut paths = vec![first.clone()];

    extend_from_cli_path_env(&mut paths, Some(path_env));

    assert!(paths.contains(&second));
    assert_eq!(paths.iter().filter(|path| *path == &first).count(), 1);
}

#[test]
fn child_search_paths_include_existing_children_with_suffix() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let base = temp.path().join("node");
    let bin = base.join("25.8.0").join("bin");
    std::fs::create_dir_all(&bin).expect("version bin should be created");

    let mut paths = Vec::new();
    extend_existing_child_search_paths(&mut paths, &base, Some("bin"));

    assert!(paths.contains(&bin));
}

#[test]
fn env_child_dir_appends_child_and_dedupes() {
    let base = std::ffi::OsString::from("/custom/toolchain");
    let mut paths = Vec::new();

    push_env_child_dir(&mut paths, Some(base.clone()), "bin");
    push_env_child_dir(&mut paths, Some(base), "bin");

    assert_eq!(paths, vec![PathBuf::from("/custom/toolchain").join("bin")]);
}

#[cfg(target_os = "windows")]
#[test]
fn cli_path_env_skips_windows_apps_alias_dir() {
    assert!(is_windows_app_execution_alias_dir(Path::new(
        r"C:\Users\tester\AppData\Local\Microsoft\WindowsApps"
    )));
    assert!(!is_windows_app_execution_alias_dir(Path::new(
        r"C:\Users\tester\AppData\Roaming\npm"
    )));
}

#[test]
fn mise_node_search_paths_include_shims_and_installed_node_bins() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let home = temp.path();
    let node_bin = home
        .join(".local/share/mise/installs/node/25.8.0")
        .join("bin");
    std::fs::create_dir_all(&node_bin).expect("node bin should be created");

    let mut paths = Vec::new();
    extend_mise_node_search_paths(&mut paths, home);

    assert!(paths.contains(&home.join(".local/share/mise/shims")));
    assert!(paths.contains(&node_bin));
}

#[cfg(not(target_os = "windows"))]
#[test]
fn tool_executable_candidates_non_windows_uses_plain_binary_name() {
    let dir = PathBuf::from("/usr/local/bin");
    let candidates = tool_executable_candidates("opencode", &dir);

    assert_eq!(candidates, vec![PathBuf::from("/usr/local/bin/opencode")]);
}

#[cfg(target_os = "windows")]
#[test]
fn tool_executable_candidates_windows_includes_cmd_exe_and_plain_name() {
    let dir = PathBuf::from("C:\\tools");
    let candidates = tool_executable_candidates("opencode", &dir);

    assert_eq!(
        candidates,
        vec![
            PathBuf::from("C:\\tools\\opencode.cmd"),
            PathBuf::from("C:\\tools\\opencode.exe"),
            PathBuf::from("C:\\tools\\opencode"),
        ]
    );
}
