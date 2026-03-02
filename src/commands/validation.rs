use std::io::Write;

use ion_skill::validate::ValidationReport;

pub fn print_validation_report(skill_name: &str, report: &ValidationReport) {
    println!("  Validation findings for '{skill_name}':");
    for finding in &report.findings {
        println!(
            "    {} [{}] {}",
            finding.severity, finding.checker, finding.message
        );
        if let Some(detail) = &finding.detail {
            println!("      {detail}");
        }
    }
    println!(
        "  Found: {} error(s), {} warning(s), {} info",
        report.error_count, report.warning_count, report.info_count
    );
}

pub fn confirm_install_on_warnings() -> anyhow::Result<bool> {
    print!("Install anyway? [y/N] ");
    std::io::stdout().flush()?;

    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();

    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}
