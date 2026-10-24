//! Build tasks for skon's Markdown Viewer.

mod icon;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Bundle file name, bundle identifier and executable name (STANDARD §2).
const BUNDLE_NAME: &str = "skons_markdown_viewer";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const TARGET: &str = "aarch64-apple-darwin";

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  bundle [--release]          build the app and assemble the .app bundle
  run [--release] [FILE...]   bundle, then start the app (optionally with files)
  dist                        release bundle plus installer and uninstaller packages
  notarize                    notarize and staple the packages in target/dist
  icon                        regenerate packaging/AppIcon.icns
  version                     print the version

environment:
  SMV_SIGN_APP_IDENTITY   Developer ID Application identity (default: ad-hoc signature)
  SMV_SIGN_PKG_IDENTITY   Developer ID Installer identity (default: unsigned packages)
  SMV_NOTARY_PROFILE      notarytool keychain profile, or
  SMV_NOTARY_KEY, SMV_NOTARY_KEY_ID, SMV_NOTARY_ISSUER   App Store Connect API key";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let release = args.iter().any(|a| a == "--release");
    let result = match args.first().map(String::as_str) {
        Some("bundle") => bundle(release).map(|app| println!("{}", app.display())),
        Some("run") => run(release, &args[1..]),
        Some("dist") => dist(),
        Some("notarize") => notarize(),
        Some("icon") => icon::generate(&root().join("packaging")),
        Some("version") => {
            println!("{VERSION}");
            Ok(())
        }
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace root")
        .to_path_buf()
}

fn run_command(command: &mut Command) -> Result {
    let status = command
        .status()
        .map_err(|e| format!("cannot run {command:?}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} failed with {status}"))
    }
}

fn command_output(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .map_err(|e| format!("cannot run {command:?}: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(text)
    } else {
        Err(format!(
            "{command:?} failed with {}:\n{text}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn copy_dir(from: &Path, to: &Path) -> Result {
    fs::create_dir_all(to).map_err(|e| format!("cannot create {}: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("cannot read {}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("cannot copy {:?}: {e}", entry.path()))?;
        }
    }
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result {
    fs::write(path, contents).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Builds the executable and assembles `skons_markdown_viewer.app`.
fn bundle(release: bool) -> Result<PathBuf> {
    let root = root();
    let profile = if release { "release" } else { "debug" };
    let mut cargo = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cargo.args(["build", "--package", BUNDLE_NAME, "--target", TARGET]);
    if release {
        cargo.arg("--release");
    }
    run_command(&mut cargo)?;

    let build_dir = root.join("target").join(TARGET).join(profile);
    let app = build_dir.join("bundle").join(format!("{BUNDLE_NAME}.app"));
    if app.exists() {
        fs::remove_dir_all(&app).map_err(|e| format!("cannot remove {}: {e}", app.display()))?;
    }
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    for dir in [&macos, &resources] {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }

    fs::copy(build_dir.join(BUNDLE_NAME), macos.join(BUNDLE_NAME))
        .map_err(|e| format!("cannot copy the executable: {e}"))?;
    let packaging = root.join("packaging");
    let info = fs::read_to_string(packaging.join("Info.plist")).map_err(|e| e.to_string())?;
    write(
        &contents.join("Info.plist"),
        &info.replace("@VERSION@", VERSION),
    )?;
    write(&contents.join("PkgInfo"), "APPL????")?;
    fs::copy(
        packaging.join("AppIcon.icns"),
        resources.join("AppIcon.icns"),
    )
    .map_err(|e| format!("cannot copy the icon: {e}"))?;
    copy_dir(&packaging.join("en.lproj"), &resources.join("en.lproj"))?;

    sign_app(&app)?;
    Ok(app)
}

/// Signs with the Developer ID Application identity when configured,
/// otherwise ad hoc.
fn sign_app(app: &Path) -> Result {
    let mut codesign = Command::new("codesign");
    codesign.args(["--force", "--sign"]);
    match env_value("SMV_SIGN_APP_IDENTITY") {
        Some(identity) => codesign.args([identity.as_str(), "--timestamp", "--options", "runtime"]),
        None => codesign.arg("-"),
    };
    run_command(codesign.arg(app))?;
    run_command(
        Command::new("codesign")
            .args(["--verify", "--strict"])
            .arg(app),
    )
}

fn run(release: bool, args: &[String]) -> Result {
    let app = bundle(release)?;
    let files: Vec<PathBuf> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(|a| env::current_dir().unwrap_or_default().join(a))
        .collect();
    run_command(Command::new(app.join("Contents/MacOS").join(BUNDLE_NAME)).args(files))
}

/// Builds the release bundle and both packages into `target/dist`
/// (STANDARD §10).
fn dist() -> Result {
    let root = root();
    let app = bundle(true)?;
    let dist = root.join("target/dist");
    if dist.exists() {
        fs::remove_dir_all(&dist).map_err(|e| e.to_string())?;
    }
    let staging = dist.join("staging");
    let payload = staging.join("payload");
    fs::create_dir_all(&payload).map_err(|e| e.to_string())?;
    run_command(
        Command::new("ditto")
            .arg(&app)
            .arg(payload.join(format!("{BUNDLE_NAME}.app"))),
    )?;
    let packaging = root.join("packaging");
    let pkg_identity = env_value("SMV_SIGN_PKG_IDENTITY");

    // Installer: one component package inside a distribution package.
    let component_plist = staging.join("component.plist");
    write(&component_plist, &component_plist_xml())?;
    run_command(
        Command::new("pkgbuild")
            .arg("--root")
            .arg(&payload)
            .arg("--component-plist")
            .arg(&component_plist)
            .args(["--install-location", "/Applications"])
            .args(["--identifier", BUNDLE_NAME, "--version", VERSION])
            .arg(staging.join("app.pkg")),
    )?;
    let installer = dist.join(format!("{BUNDLE_NAME}-{VERSION}.pkg"));
    product_build(
        &packaging.join("installer"),
        &staging,
        pkg_identity.as_deref(),
        &installer,
    )?;

    // Uninstaller: two payload-free packages that only run scripts.
    for (part, identifier) in [
        ("app", format!("{BUNDLE_NAME}.uninstaller.app")),
        ("userdata", format!("{BUNDLE_NAME}.uninstaller.userdata")),
    ] {
        run_command(
            Command::new("pkgbuild")
                .arg("--nopayload")
                .arg("--scripts")
                .arg(packaging.join("uninstaller/scripts").join(part))
                .args(["--identifier", &identifier, "--version", VERSION])
                .arg(staging.join(format!("uninstall-{part}.pkg"))),
        )?;
    }
    let uninstaller = dist.join(format!("{BUNDLE_NAME}-uninstaller-{VERSION}.pkg"));
    product_build(
        &packaging.join("uninstaller"),
        &staging,
        pkg_identity.as_deref(),
        &uninstaller,
    )?;

    fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    for package in [&installer, &uninstaller] {
        println!("{}", package.display());
    }
    if pkg_identity.is_none() {
        eprintln!("note: SMV_SIGN_PKG_IDENTITY is not set; the packages are unsigned");
    }
    Ok(())
}

fn product_build(source: &Path, staging: &Path, identity: Option<&str>, output: &Path) -> Result {
    let template =
        fs::read_to_string(source.join("distribution.xml")).map_err(|e| e.to_string())?;
    let distribution = staging.join(format!(
        "{}-distribution.xml",
        source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("package")
    ));
    write(&distribution, &template.replace("@VERSION@", VERSION))?;
    let mut command = Command::new("productbuild");
    command
        .arg("--distribution")
        .arg(&distribution)
        .arg("--package-path")
        .arg(staging)
        .arg("--resources")
        .arg(source.join("resources"));
    if let Some(identity) = identity {
        command.args(["--sign", identity, "--timestamp"]);
    }
    run_command(command.arg(output))
}

/// The app must always go to /Applications, even if another copy exists
/// elsewhere (STANDARD §10).
fn component_plist_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<dict>
		<key>BundleHasStrictIdentifier</key>
		<true/>
		<key>BundleIsRelocatable</key>
		<false/>
		<key>BundleIsVersionChecked</key>
		<false/>
		<key>BundleOverwriteAction</key>
		<string>upgrade</string>
		<key>RootRelativeBundlePath</key>
		<string>{BUNDLE_NAME}.app</string>
	</dict>
</array>
</plist>
"#
    )
}

/// Notarizes and staples every package in `target/dist`.
fn notarize() -> Result {
    let credentials: Vec<String> = if let Some(profile) = env_value("SMV_NOTARY_PROFILE") {
        vec!["--keychain-profile".into(), profile]
    } else if let (Some(key), Some(key_id), Some(issuer)) = (
        env_value("SMV_NOTARY_KEY"),
        env_value("SMV_NOTARY_KEY_ID"),
        env_value("SMV_NOTARY_ISSUER"),
    ) {
        vec![
            "--key".into(),
            key,
            "--key-id".into(),
            key_id,
            "--issuer".into(),
            issuer,
        ]
    } else {
        return Err("notarization credentials are not configured (see `cargo xtask`)".into());
    };
    let dist = root().join("target/dist");
    let mut packages: Vec<PathBuf> = fs::read_dir(&dist)
        .map_err(|e| format!("cannot read {}: {e}", dist.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "pkg"))
        .collect();
    packages.sort();
    if packages.is_empty() {
        return Err("no packages in target/dist; run `cargo xtask dist` first".into());
    }
    for package in packages {
        let output = command_output(
            Command::new("xcrun")
                .args(["notarytool", "submit"])
                .arg(&package)
                .args(["--wait", "--output-format", "json"])
                .args(&credentials),
        )?;
        if !output.replace(' ', "").contains("\"status\":\"Accepted\"") {
            return Err(format!(
                "notarization of {} failed:\n{output}",
                package.display()
            ));
        }
        run_command(
            Command::new("xcrun")
                .args(["stapler", "staple"])
                .arg(&package),
        )?;
    }
    Ok(())
}
