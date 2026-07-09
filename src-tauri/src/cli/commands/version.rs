//! `os version` — 显示版本信息

use crate::output;

pub fn run(json: bool) -> Result<(), String> {
    let info = open_sunstar_lib::get_build_info();

    if json {
        output::print_result(&info, true);
    } else {
        eprintln!(
            "{} {}",
            console::style("OpenSunstar CLI (os)").cyan().bold(),
            console::style(format!("v{}", info.app_version)).green()
        );
        output::dim(&format!("Schema version: v{}", info.schema_version));
    }

    Ok(())
}
