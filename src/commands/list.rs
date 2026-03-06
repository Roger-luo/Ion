use ion_skill::manifest::Manifest;

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        println!("No skills declared in Ion.toml.");
        return Ok(());
    }

    println!("Skills ({}):", p.bold(&manifest.skills.len().to_string()));
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        let locked = lockfile.find(name);

        let is_binary = locked.and_then(|l| l.binary.as_deref()).is_some();

        let version_str = if is_binary {
            locked.and_then(|l| l.binary_version.as_deref()).unwrap_or("unknown")
        } else {
            locked.and_then(|l| l.version.as_deref()).unwrap_or("unknown")
        };

        let type_indicator = if is_binary {
            format!(" {}", p.info("(binary)"))
        } else {
            let commit_str = locked
                .and_then(|l| l.commit.as_deref())
                .map(|c| &c[..c.len().min(8)])
                .unwrap_or("none");
            format!(" {}", p.dim(&format!("({commit_str})")))
        };

        let installed = ctx
            .project_dir
            .join(".agents")
            .join("skills")
            .join(name)
            .exists();
        let status = if installed {
            p.success("installed")
        } else {
            p.warn("not installed")
        };

        let display_version = if version_str.starts_with('v') {
            version_str.to_string()
        } else {
            format!("v{version_str}")
        };
        println!("  {} {}{} [{}]",
            p.bold(name),
            p.dim(&display_version),
            type_indicator,
            status
        );
        println!("    source: {}", p.info(&source.source));
    }
    Ok(())
}
