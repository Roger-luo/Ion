use ion_skill::manifest::Manifest;

use crate::context::ProjectContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    println!("Skills ({}):", manifest.skills.len());
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        let locked = lockfile.find(name);

        let version_str = locked
            .and_then(|l| l.version.as_deref())
            .unwrap_or("unknown");
        let commit_str = locked
            .and_then(|l| l.commit.as_deref())
            .map(|c| &c[..c.len().min(8)])
            .unwrap_or("none");
        let installed = ctx
            .project_dir
            .join(".agents")
            .join("skills")
            .join(name)
            .exists();
        let status = if installed {
            "installed"
        } else {
            "not installed"
        };

        println!("  {name} v{version_str} ({commit_str}) [{status}]");
        println!("    source: {}", source.source);
    }
    Ok(())
}
