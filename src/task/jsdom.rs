//! The jsdom of a `JSDOMNodeJSEnv` test run (`teq test`): required from the package root the export
//! records (`js.env.packageRoot`), installed there from the lockfile it records as `teq dev`
//! installs a project's packages (npm's `ci`, yarn's and pnpm's frozen installs, which install the
//! lockfile as it is), and checked against the version an npm lockfile pins.

use super::dev;
use super::export::{Export, JsEnv};
use crate::lsp::json::Json;
use std::collections::BTreeMap;

/// The command that installs a lockfile as it is, by its name.
fn install_command(lockfile: &str) -> Option<Vec<String>> {
    let name = lockfile.rsplit(['/', '\\']).next().unwrap_or(lockfile);
    let words: &[&str] = match name {
        "package-lock.json" | "npm-shrinkwrap.json" => &["npm", "ci", "--no-audit", "--no-fund"],
        "yarn.lock" => &["yarn", "install", "--frozen-lockfile"],
        "pnpm-lock.yaml" => &["pnpm", "install", "--frozen-lockfile"],
        _ => return None,
    };
    Some(words.iter().map(|w| w.to_string()).collect())
}

/// The version of jsdom an npm lockfile installs at the package root (`packages`, versions 2
/// and 3 of its format).
fn pinned(lockfile: &Json) -> Option<String> {
    lockfile.get("packages")?.get("node_modules/jsdom")?.get("version")?.str().map(str::to_string)
}

/// jsdom ready at the environment's package root: its lockfile installed unless it is the one
/// installed last, and the version installed the one the lockfile pins.
pub fn ready(export: &Export, env: &JsEnv, environment: &BTreeMap<String, String>, progress: &dyn Fn(&str)) -> Result<(), String> {
    let root = export.path(env.package_root.as_deref().unwrap_or("."));
    let lockfile = env.lockfile.as_deref().unwrap_or("");
    if lockfile.is_empty() {
        return Err(format!("the export names no lockfile that installs jsdom at {}: export it again", root.display()));
    }
    let command = install_command(lockfile).ok_or_else(|| format!("{} is no lockfile of npm, yarn or pnpm, which install jsdom for Test / jsEnv", lockfile))?;
    dev::install(export, &command, lockfile, &root, Some(environment), progress)?;
    let manifest = root.join("node_modules/jsdom/package.json");
    let installed = std::fs::read_to_string(&manifest).ok().and_then(|t| Json::parse(&t).ok()).and_then(|j| j.get("version").and_then(Json::str).map(str::to_string));
    let Some(installed) = installed else {
        return Err(format!("{} installs no jsdom at {}, which Test / jsEnv (JSDOMNodeJSEnv) requires", lockfile, root.display()));
    };
    if command[0] == "npm" {
        let lock = std::fs::read_to_string(export.path(lockfile)).ok().and_then(|t| Json::parse(&t).ok());
        match lock.as_ref().and_then(pinned) {
            Some(p) if p == installed => {}
            Some(p) => return Err(format!("{} pins jsdom {}, and {} holds {}: install it again ({})", lockfile, p, manifest.display(), installed, command.join(" "))),
            None => return Err(format!("{} pins no jsdom, which Test / jsEnv (JSDOMNodeJSEnv) requires", lockfile)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lockfile_names_its_install() {
        assert_eq!(install_command("web/package-lock.json").unwrap()[..2], ["npm", "ci"]);
        assert_eq!(install_command("yarn.lock").unwrap()[2], "--frozen-lockfile");
        assert!(install_command("package.json").is_none());
        let lock = Json::parse(r#"{"lockfileVersion":3,"packages":{"":{},"node_modules/jsdom":{"version":"27.4.0"}}}"#).unwrap();
        assert_eq!(pinned(&lock).as_deref(), Some("27.4.0"));
    }
}
