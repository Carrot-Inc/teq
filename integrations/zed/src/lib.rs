mod lock;

use std::cmp::Ordering;
use std::fs;
use std::io::Read;
use std::path::Path;
use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, LanguageServerInstallationStatus, Os, Result,
    http_client::{HttpMethod, HttpRequest, RedirectPolicy},
    lsp::{Symbol, SymbolKind},
};

/// The published teq the extension fetches when no other binary is found: the newest release of
/// `teq` for the platform (docs/TARGETS.md, "Releases"), teq's GitHub release, found through GitHub's
/// releases API (`GITHUB_API`): the latest release, or, when there is no release but pre-releases,
/// the newest of those by its tag's version; its asset `teq-<version>-<classifier>` (`asset_name`:
/// `.exe` for Windows alone) and `SHA256SUMS` by their exact names, the binary checked against the
/// SHA-256 there. What a search found is kept with the time it was made (`METADATA`) and asked
/// again only a day later (`FRESH`) or when its copy is missing; GitHub's limit of an address's
/// requests, reached, leaves the copies and what was found before serving. A release never
/// changes: each one fetched is kept as `teq-<version>/teq` with a stamp of its digest and size, and
/// the newest copy serves when GitHub is out of reach. A copy runs only when its bytes' digest and
/// size, read again at each start, are its release's (`verified`): a file changed in place is no
/// copy, whatever its size. The releases before `FIRST_RELEASE` are not served: neither a lock
/// pinning one nor a copy of one runs.
const GITHUB_API: &str = "https://api.github.com/repos/Carrot-Inc/teq";
const GITHUB_RELEASES: &str = "https://github.com/Carrot-Inc/teq/releases/download";
/// The first release served, by its numbers: the first of GitHub's.
const FIRST_RELEASE: [u64; 3] = [0, 1, 7];
/// What the last search of GitHub found, with its time, in the work directory.
const METADATA: &str = "github-release.json";
/// How long, in seconds, a search of GitHub stands before it is made again.
const FRESH: u64 = 24 * 60 * 60;
/// How many pages of GitHub's release listing are read for a pre-release, a hundred releases each.
const LIST_PAGES: usize = 3;
/// How many times a request GitHub does not answer is made, at most.
const TRIES: usize = 2;
/// How large an answer of GitHub's may be.
const PAGE_BYTES: usize = 1 << 20;
/// Where the export lies without the native driver, under the worktree's root.
const UNDER_TARGET: &str = "target/teq/teq.lock";

struct TeqExtension;

impl zed::Extension for TeqExtension {
    fn new() -> Self {
        TeqExtension
    }

    // Zed applies the `lsp.teq.binary` settings itself: with a `path` it starts that binary with
    // the settings' `arguments` and never asks here; without one it asks and then puts the
    // settings' `arguments`, when given, over the ones returned. Asked, the binary is `TEQ` of the
    // worktree's shell environment, else the one the worktree's lock pins for the platform,
    // else the one on the PATH, else the extension's own copy of the newest release GitHub
    // serves.
    fn language_server_command(&mut self, id: &zed::LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
        let env = worktree.shell_env();
        let named = env.iter().find(|(k, v)| k == "TEQ" && !v.is_empty()).map(|(_, v)| v.clone());
        let pinned = match named {
            Some(named) => Some(named),
            None => self.pinned_binary(id, worktree).map_err(|why| self.failed(id, why))?,
        };
        let command = match pinned.or_else(|| worktree.which("teq")) {
            Some(found) => found,
            None => self.fetched_binary(id)?,
        };
        Ok(zed::Command { command, args: vec!["lsp".to_string()], env })
    }

    /// The settings' `lsp.teq.initialization_options` (`maxSessions`, `sessionIdleSeconds`), passed
    /// to the server as they are.
    fn language_server_initialization_options(&mut self, _id: &zed::LanguageServerId, worktree: &zed::Worktree) -> Result<Option<serde_json::Value>> {
        Ok(zed::settings::LspSettings::for_worktree("teq", worktree).ok().and_then(|s| s.initialization_options))
    }

    /// A workspace symbol shown as its declaration reads (`object Foo`, `def bar`) and matched by
    /// its name alone, with the kinds the server gives (`src/typer/index.rs`: object 2, class 5,
    /// def 6, val 8, enum 10, trait 11, var 13, enum case 22, type alias 26).
    fn label_for_symbol(&self, _id: &zed::LanguageServerId, symbol: Symbol) -> Option<zed::CodeLabel> {
        let prefix = match symbol.kind {
            SymbolKind::Module => "object ",
            SymbolKind::Class => "class ",
            SymbolKind::Interface => "trait ",
            SymbolKind::Enum => "enum ",
            SymbolKind::EnumMember => "case ",
            SymbolKind::Method | SymbolKind::Function => "def ",
            SymbolKind::Variable => "var ",
            SymbolKind::Field | SymbolKind::Property | SymbolKind::Constant => "val ",
            SymbolKind::TypeParameter => "type ",
            _ => "",
        };
        let name = symbol.name;
        let code = format!("{prefix}{name}");
        let code_len = code.len();
        Some(zed::CodeLabel { code, spans: vec![zed::CodeLabelSpan::code_range(0..code_len)], filter_range: (prefix.len()..code_len).into() })
    }
}

impl TeqExtension {
    /// The binary the worktree's lock pins for the platform (`teq.lock` at its root, else
    /// `target/teq/teq.lock`, where the export writes it without the native driver), in the work directory
    /// as `teq-<sha1>/teq` (`teq.exe` on Windows): fetched from its URL when missing, kept only
    /// when its size and sha1 are the pinned ones. Nothing when the worktree has no lock, the lock
    /// is not one this copy of teq's reader reads (`lock.rs`) or pins no binary for the platform, or
    /// the fetch fails (a repository that wants credentials, which the extension cannot read): the
    /// PATH and the release's copy follow. A lock of a compiler before `FIRST_RELEASE` is refused, as
    /// the launchers refuse it, and nothing else runs in its place.
    fn pinned_binary(&mut self, id: &zed::LanguageServerId, worktree: &zed::Worktree) -> Result<Option<String>> {
        let classifier = classifier_for(zed::current_platform());
        let Some((file, text)) = [lock::FILE, UNDER_TARGET].into_iter().find_map(|file| worktree.read_text_file(file).ok().map(|text| (file, text))) else { return Ok(None) };
        let pin = match pin_in(&text, &classifier) {
            Ok(Some(pin)) => pin,
            Ok(None) => return Ok(None),
            Err(compiler) => return Err(format!("{file} pins teq {compiler}, and releases before 0.1.7 are not served: pin 0.1.7 or later (sbt teqExportAll), or set TEQ to a local binary")),
        };
        let dir = format!("teq-{}", pin.sha1);
        let path = format!("{dir}/{}", binary_name(&classifier));
        if fs::metadata(&path).is_ok_and(|m| m.len() == pin.size) {
            return Ok(Some(path));
        }
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Downloading);
        let partial = format!("{dir}/teq.download");
        let fetched = fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|()| zed::download_file(&pin.url, &partial, DownloadedFileType::Uncompressed))
            .and_then(|()| match sha1_of_file(&partial) {
                Ok((sha1, size)) if sha1 == pin.sha1 && size == pin.size => Ok(()),
                Ok((sha1, size)) => Err(format!("{} is {size} bytes with sha1 {sha1}, the export pins {} bytes with sha1 {}", pin.url, pin.size, pin.sha1)),
                Err(e) => Err(e.to_string()),
            })
            .and_then(|()| zed::make_file_executable(&partial))
            .and_then(|()| fs::rename(&partial, &path).map_err(|e| e.to_string()));
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
        match fetched {
            Ok(()) => Ok(Some(path)),
            Err(_) => {
                let _ = fs::remove_dir_all(&dir);
                Ok(None)
            }
        }
    }

    /// The extension's own copy of the newest release's binary in the work directory,
    /// `teq-<version>/teq` (`teq.exe` on Windows): reused when its digest and size are the ones the search lists, else fetched; when
    /// the search gives no release (GitHub out of reach, an answer of another shape, no release for
    /// the platform) or the fetch fails, the newest copy present serves.
    fn fetched_binary(&mut self, id: &zed::LanguageServerId) -> Result<String> {
        let classifier = classifier_for(zed::current_platform());
        let present = |release: &Release| verified(Path::new(&release.path(&classifier)), &release.digest, release.size);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let kept = fs::read_to_string(METADATA).ok().and_then(|text| cached_in(&text, &classifier));
        let found = match kept {
            // A search of the last day stands while its copy is the release's: no request, and the copy runs.
            Some((at, release)) if now.saturating_sub(at) < FRESH && present(&release) => return Ok(release.path(&classifier)),
            kept => {
                zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::CheckingForUpdate);
                match discover(&fetch_api, &fetch_file, &classifier) {
                    Ok(found) => {
                        if let Some(release) = found.first() {
                            let _ = fs::write(METADATA, cached_text(now, release, &classifier));
                        }
                        Ok(found)
                    }
                    // GitHub out of reach or refusing more requests for now: what the last search found stands.
                    Err(e) => kept.map(|(_, release)| vec![release]).ok_or(e),
                }
            }
        };
        let path = match plan(found, &present, &|| newest_copy(Path::new("."), &classifier), &classifier) {
            Plan::Reuse(path) | Plan::Serve(path) => path,
            Plan::Fetch(release) => {
                // A copy of the release of other bytes than the search lists is no copy of it.
                if let Err(e) = fs::remove_dir_all(release.dir()) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(self.failed(id, format!("teq is not on the PATH and the copy of {} of other bytes than the repository lists could not be removed ({e}): {ALTERNATIVES}", release.dir())));
                    }
                }
                zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Downloading);
                match fetch_release(&release, &classifier) {
                    Ok(path) => {
                        remove_other_copies(&release.dir());
                        path
                    }
                    Err(e) => match newest_copy(Path::new("."), &classifier) {
                        Some(copy) => copy,
                        None => return Err(self.failed(id, format!("teq is not on the PATH and {} could not be fetched ({e}): {ALTERNATIVES}", release.url(&classifier)))),
                    },
                }
            }
            Plan::Fail(why) => return Err(self.failed(id, format!("teq is not on the PATH and {why}: {ALTERNATIVES}"))),
        };
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
        Ok(path)
    }

    fn failed(&self, id: &zed::LanguageServerId, message: String) -> String {
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Failed(message.clone()));
        message
    }
}

/// A small file at a URL (a release's `SHA256SUMS`), its redirects followed (a release's asset is
/// served from another host): an answer 4xx or 5xx fails, naming its status code, as Zed's `fetch`
/// fails it.
fn fetch_file(url: &str) -> Result<String> {
    let request = HttpRequest::builder().method(HttpMethod::Get).url(url).redirect_policy(RedirectPolicy::FollowLimit(5)).build()?;
    let body = request.fetch()?.body;
    if body.len() > PAGE_BYTES {
        return Err(format!("{url} answers more than {PAGE_BYTES} bytes"));
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// An answer of GitHub's API, asked as its documentation asks (a User-Agent, its media type), made
/// again once when it is not answered; a 4xx or 5xx fails naming its status code.
fn fetch_api(url: &str) -> Result<String> {
    let mut last = String::new();
    for _ in 0..TRIES {
        let request = HttpRequest::builder()
            .method(HttpMethod::Get)
            .url(url)
            .header("User-Agent", "teq-zed-extension")
            .header("Accept", "application/vnd.github+json")
            .redirect_policy(RedirectPolicy::FollowLimit(3))
            .build()?;
        match request.fetch() {
            Ok(response) if response.body.len() > PAGE_BYTES => return Err(format!("{url} answers more than {PAGE_BYTES} bytes")),
            Ok(response) => return Ok(String::from_utf8_lossy(&response.body).into_owned()),
            // An answer with a status is GitHub's say, not a failure to reach it: not asked again.
            Err(e) if e.contains("status code") => return Err(e),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Whether a failed request (`fetch_file`'s, `fetch_api`'s) is GitHub's answer that the file is not
/// there: Zed's `fetch` fails a request answered 4xx or 5xx, naming the status code.
fn not_found(error: &str) -> bool {
    error.contains("status code 404")
}

/// Whether a failed request is GitHub refusing more of this address's requests for now (403 or 429).
fn rate_limited(error: &str) -> bool {
    error.contains("status code 403") || error.contains("status code 429")
}

const ALTERNATIVES: &str = "put teq on the PATH, or set lsp.teq.binary in Zed's settings to its path with the arguments [\"lsp\"]";

/// What a binary's name ends in on the classifier's platform: `.exe` on Windows, whose process
/// creation looks for it, nothing elsewhere; the release's assets and the extension's copies are named
/// by it (sbt-teq's `Release.executableSuffix` and bench/ship-release.sh's `release_asset` have the
/// same rule).
fn executable_suffix(classifier: &str) -> &'static str {
    if classifier.starts_with("windows-") {
        ".exe"
    } else {
        ""
    }
}

/// The name of the extension's copy of the binary: `teq.exe` on Windows, as sbt-teq names its copy;
/// `teq` elsewhere.
fn binary_name(classifier: &str) -> &'static str {
    if executable_suffix(classifier).is_empty() {
        "teq"
    } else {
        "teq.exe"
    }
}

/// A binary's name among a GitHub release's assets: `teq-<version>-<classifier>`, `.exe` for Windows
/// alone.
fn asset_name(version: &str, classifier: &str) -> String {
    format!("teq-{version}-{classifier}{}", executable_suffix(classifier))
}

/// The binary's classifier for the platform, in sbt-teq's convention (that of protoc's
/// artifacts): the released binaries are the classifiers that exist.
fn classifier_for((os, arch): (Os, Architecture)) -> String {
    let os = match os {
        Os::Mac => "osx",
        Os::Linux => "linux",
        Os::Windows => "windows",
    };
    let arch = match arch {
        Architecture::Aarch64 => "aarch_64",
        Architecture::X8664 => "x86_64",
        Architecture::X86 => "x86",
    };
    format!("{os}-{arch}")
}

/// A release's version: `<major>.<minor>.<patch>`, with a pre-release suffix `-<ident>(.<ident>)*`
/// of alphanumeric identifiers or without, none of them `SNAPSHOT`. Ordered as semver orders:
/// the core numerically, a suffix below none, identifiers pairwise (a number numerically and
/// before a word, a word lexically, a shorter list below its extension).
#[derive(Debug, Clone)]
struct Version {
    core: [u64; 3],
    pre: Option<Vec<Ident>>,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Ident {
    Number(u64),
    Word(String),
}

impl Version {
    fn parse(text: &str) -> Option<Version> {
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut numbers = core.split('.').map(number);
        let core = [numbers.next()??, numbers.next()??, numbers.next()??];
        if numbers.next().is_some() {
            return None;
        }
        let pre = match pre {
            Some(pre) => Some(pre.split('.').map(ident).collect::<Option<Vec<_>>>()?),
            None => None,
        };
        Some(Version { core, pre, text: text.to_string() })
    }
}

/// A number of a version: digits alone.
fn number(s: &str) -> Option<u64> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

/// An identifier of a pre-release suffix: alphanumeric, not `SNAPSHOT`, a number when digits alone.
fn ident(s: &str) -> Option<Ident> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_alphanumeric()) || s.eq_ignore_ascii_case("SNAPSHOT") {
        return None;
    }
    match number(s) {
        Some(n) => Some(Ident::Number(n)),
        None if s.bytes().all(|b| b.is_ascii_digit()) => None,
        None => Some(Ident::Word(s.to_string())),
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| match (&self.pre, &other.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

/// A release's binary for the platform as the search found it: its version, its digest (the SHA-256
/// the release's `SHA256SUMS` gives) and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Release {
    version: Version,
    digest: String,
    size: u64,
}

impl Release {
    fn dir(&self) -> String {
        format!("teq-{}", self.version.text)
    }

    fn path(&self, classifier: &str) -> String {
        format!("{}/{}", self.dir(), binary_name(classifier))
    }

    /// The binary's URL, the GitHub release's canonical asset URL, composed from the version and the
    /// classifier.
    fn url(&self, classifier: &str) -> String {
        format!("{GITHUB_RELEASES}/v{}/{}", self.version.text, asset_name(&self.version.text, classifier))
    }
}

/// The releases of teq for the classifier: GitHub's (`github_release`), its API asked by `api` and its
/// `SHA256SUMS` read by `file`; none when GitHub gives none.
fn discover(api: &dyn Fn(&str) -> Result<String>, file: &dyn Fn(&str) -> Result<String>, classifier: &str) -> Result<Vec<Release>> {
    Ok(github_release(api, file, classifier)?.into_iter().collect())
}

/// GitHub's release of teq for the classifier: the latest release (`/releases/latest`, which GitHub
/// gives neither a draft nor a pre-release), and only when there is none, the newest pre-release that
/// is no draft, by its tag's version, of the first `LIST_PAGES` pages of the release listing. A
/// release found counts when its tag is `v<version>` of a release's version and it holds the assets
/// `teq-<version>-<classifier>` (`asset_name`) and `SHA256SUMS`, whose one line for the binary gives its SHA-256;
/// a latest release without them gives none (an older release is not looked for, a pre-release not
/// taken in its place). GitHub refusing more requests (403, 429) is a failure saying so.
fn github_release(api: &dyn Fn(&str) -> Result<String>, file: &dyn Fn(&str) -> Result<String>, classifier: &str) -> Result<Option<Release>> {
    let refused = |e: String| if rate_limited(&e) { format!("GitHub refuses more requests from this address for now ({e})") } else { e };
    let parse = |text: &str| serde_json::from_str::<serde_json::Value>(text).map_err(|e| format!("GitHub's answer is no JSON ({e})"));
    match api(&format!("{GITHUB_API}/releases/latest")) {
        Ok(text) => return release_of(&parse(&text)?, file, classifier),
        Err(e) if not_found(&e) => {}
        Err(e) => return Err(refused(e)),
    }
    let mut newest: Option<(Version, serde_json::Value)> = None;
    for page in 1..=LIST_PAGES {
        let listing = parse(&api(&format!("{GITHUB_API}/releases?per_page=100&page={page}")).map_err(refused)?)?;
        let items = listing.as_array().ok_or("GitHub's release listing is no list")?;
        for release in items {
            let flag = |key: &str| release.get(key).and_then(|v| v.as_bool());
            if flag("draft") != Some(false) || flag("prerelease") != Some(true) {
                continue;
            }
            let Some(version) = release.get("tag_name").and_then(|t| t.as_str()).and_then(|t| t.strip_prefix('v')).and_then(Version::parse) else { continue };
            if !served(&version.text) {
                continue;
            }
            if newest.as_ref().is_none_or(|(v, _)| version > *v) {
                newest = Some((version, release.clone()));
            }
        }
        if items.len() < 100 {
            break;
        }
    }
    match newest {
        Some((_, release)) => release_of(&release, file, classifier),
        None => Ok(None),
    }
}

/// A GitHub release's binary for the classifier, from the release's JSON: its version from its tag,
/// its asset by its exact name with the size GitHub lists, and its SHA-256 from `SHA256SUMS`, read
/// from the release's canonical directory.
fn release_of(release: &serde_json::Value, file: &dyn Fn(&str) -> Result<String>, classifier: &str) -> Result<Option<Release>> {
    let Some(version) = release.get("tag_name").and_then(|t| t.as_str()).and_then(|t| t.strip_prefix('v')).and_then(Version::parse) else { return Ok(None) };
    if !served(&version.text) {
        return Ok(None);
    }
    let asset = asset_name(&version.text, classifier);
    let assets = release.get("assets").and_then(|a| a.as_array()).map_or(&[][..], |a| a.as_slice());
    let named = |name: &str| assets.iter().find(|a| a.get("name").and_then(|n| n.as_str()) == Some(name));
    let (Some(binary), Some(_)) = (named(&asset), named("SHA256SUMS")) else { return Ok(None) };
    let Some(size) = binary.get("size").and_then(|s| s.as_u64()) else { return Ok(None) };
    let sums = file(&format!("{GITHUB_RELEASES}/v{}/SHA256SUMS", version.text))?;
    let lines: Vec<&str> = sums
        .lines()
        .filter_map(|line| {
            let (digest, name) = line.trim().split_once(char::is_whitespace)?;
            (name.trim().trim_start_matches('*') == asset).then_some(digest)
        })
        .collect();
    let &[digest] = lines.as_slice() else { return Err(format!("SHA256SUMS of v{} names {asset} {} times", version.text, lines.len())) };
    let digest = digest.to_ascii_lowercase();
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("SHA256SUMS of v{} gives {asset} no SHA-256", version.text));
    }
    Ok(Some(Release { version, digest, size }))
}

/// What the last search of GitHub found for the classifier, from `METADATA`'s text: the time it was
/// made and the release.
fn cached_in(text: &str, classifier: &str) -> Option<(u64, Release)> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    (value.get("classifier")?.as_str()? == classifier).then_some(())?;
    let digest = value.get("sha256")?.as_str()?.to_ascii_lowercase();
    (digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())).then_some(())?;
    let release = Release { version: Version::parse(value.get("version")?.as_str()?)?, digest, size: value.get("size")?.as_u64()? };
    served(&release.version.text).then_some(())?;
    Some((value.get("fetched")?.as_u64()?, release))
}

fn cached_text(at: u64, release: &Release, classifier: &str) -> String {
    serde_json::json!({"fetched": at, "classifier": classifier, "version": release.version.text, "sha256": release.digest, "size": release.size}).to_string()
}

/// What a start does for the fallback, from the releases the search found and the copies present:
/// reuse the newest release's copy, fetch it, serve the newest copy (`copy`, asked only then) when the
/// search gave no release, or fail.
#[derive(Debug, PartialEq, Eq)]
enum Plan {
    Reuse(String),
    Fetch(Release),
    Serve(String),
    Fail(String),
}

fn plan(found: Result<Vec<Release>>, present: &dyn Fn(&Release) -> bool, copy: &dyn Fn() -> Option<String>, classifier: &str) -> Plan {
    match found.map(|releases| releases.into_iter().max_by(|a, b| a.version.cmp(&b.version))) {
        Ok(Some(release)) if present(&release) => Plan::Reuse(release.path(classifier)),
        Ok(Some(release)) => Plan::Fetch(release),
        Ok(None) => copy().map_or_else(|| Plan::Fail(format!("the repository lists no release of teq for {classifier}")), Plan::Serve),
        Err(e) => copy().map_or_else(|| Plan::Fail(format!("the releases of teq could not be listed ({e})")), Plan::Serve),
    }
}

/// Fetches a release's binary into `teq-<version>/teq` (`teq.exe` on Windows): under another name first (Zed's
/// download following the asset's redirect), checked against the digest and size the search found (the
/// SHA-256 of the release's `SHA256SUMS`), made executable, stamped,
/// and renamed last, so that a start interrupted mid-download leaves no file a later start takes for
/// the binary, and nothing unchecked runs; a failure removes the directory.
fn fetch_release(release: &Release, classifier: &str) -> Result<String> {
    let dir = release.dir();
    let path = release.path(classifier);
    let partial = format!("{dir}/teq.download");
    let url = release.url(classifier);
    let fetched = fs::create_dir_all(&dir)
        .map_err(|e| e.to_string())
        .and_then(|()| zed::download_file(&url, &partial, DownloadedFileType::Uncompressed))
        .and_then(|()| match digest_of_file(&partial, release.digest.len()) {
            Ok((digest, size)) if digest == release.digest && size == release.size => Ok(()),
            Ok((digest, size)) => Err(format!("{url} is {size} bytes with digest {digest}, the release gives {} bytes with {}", release.size, release.digest)),
            Err(e) => Err(e.to_string()),
        })
        .and_then(|()| zed::make_file_executable(&partial))
        .and_then(|()| fs::write(format!("{dir}/stamp"), format!("{} {}\n", release.digest, release.size)).map_err(|e| e.to_string()))
        .and_then(|()| fs::rename(&partial, &path).map_err(|e| e.to_string()));
    if let Err(e) = fetched {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(path)
}

/// The newest release among the extension's copies in a directory: a `teq-<version>/teq`
/// (`binary_name`) whose digest and size are its stamp's (`verified`), as the binary's path relative
/// to the directory; a newer copy of other bytes or without a stamp is removed on the way. The pins'
/// `teq-<sha1>/` and the snapshot's directory of before are no release's, and a copy of a release
/// before `FIRST_RELEASE` none that serves: it stays until a fetch removes the other copies.
fn newest_copy(dir: &Path, classifier: &str) -> Option<String> {
    let binary_name = binary_name(classifier);
    let mut copies: Vec<(Version, String)> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let version = name.strip_prefix("teq-").and_then(Version::parse).filter(|v| v.core >= FIRST_RELEASE)?;
            entry.path().join(binary_name).is_file().then_some((version, name))
        })
        .collect();
    copies.sort_by(|a, b| b.0.cmp(&a.0));
    copies.into_iter().find_map(|(_, name)| {
        let copy = dir.join(&name);
        let stamp = fs::read_to_string(copy.join("stamp")).ok().and_then(|text| stamp_in(&text));
        if stamp.is_some_and(|(digest, size)| verified(&copy.join(binary_name), &digest, size)) {
            Some(format!("{name}/{binary_name}"))
        } else {
            let _ = fs::remove_dir_all(&copy);
            None
        }
    })
}

/// Whether the file at a path is a release's binary: its digest (SHA-256 or SHA-1, by the given
/// one's length) and its size, read now, the given ones. Its size is asked first, so that a copy of
/// another size is not read.
fn verified(path: &Path, digest: &str, size: u64) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == size)
        && digest_of_file(path, digest.len()).is_ok_and(|(d, s)| d == digest && s == size)
}

/// A copy's stamp: the digest (SHA-256 or SHA-1) and the size the search found when it was fetched.
fn stamp_in(text: &str) -> Option<(String, u64)> {
    let mut words = text.split_whitespace();
    let digest = words.next()?;
    ((digest.len() == 40 || digest.len() == 64) && digest.bytes().all(|b| b.is_ascii_hexdigit())).then_some(())?;
    Some((digest.to_ascii_lowercase(), words.next()?.parse().ok()?))
}

/// Removes the extension's copies of other releases beside `keep` in the work directory,
/// `teq-<version>/`, and the snapshot's of before, `teq-snapshot/`; the pinned binaries'
/// `teq-<sha1>/` stay.
fn remove_other_copies(keep: &str) {
    let Ok(entries) = fs::read_dir(".") else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix("teq-") else { continue };
        let pinned = rest.len() == 40 && rest.bytes().all(|b| b.is_ascii_hexdigit());
        if name != keep && !pinned {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// The binary a lock pins for a classifier: the fields of its `binaries` line, its URL, sha1 and
/// size. A lock whose compiler (its `teq:` line) is not `served` pins none.
#[derive(Debug, PartialEq)]
struct Pin {
    url: String,
    sha1: String,
    size: u64,
}

/// The lock's pin for the classifier: none when the lock is not one this reader reads, names a
/// compiler of no release's version or pins no binary for the classifier; the compiler, refused,
/// when it is a release before `FIRST_RELEASE`.
fn pin_in(text: &str, classifier: &str) -> std::result::Result<Option<Pin>, String> {
    let Ok((compiler, format)) = lock::header(text) else { return Ok(None) };
    if format != lock::FORMAT {
        return Ok(None);
    }
    if before_first(&compiler) {
        return Err(compiler);
    }
    Ok(served(&compiler).then(|| pinned_in(text, classifier)).flatten())
}

fn pinned_in(text: &str, classifier: &str) -> Option<Pin> {
    let value = lock::parse(text).ok()?;
    let fields: Vec<&str> = value.get("binaries")?.get(classifier)?.str()?.split(' ').collect();
    let &[url, sha1, size] = fields.as_slice() else { return None };
    let sha1 = sha1.to_ascii_lowercase();
    (sha1.len() == 40 && sha1.bytes().all(|b| b.is_ascii_hexdigit())).then_some(())?;
    Some(Pin { url: url.to_string(), sha1, size: size.parse().ok()? })
}

/// Whether a lock's compiler is one the extension runs: `FIRST_RELEASE` or later by its numbers, a
/// pre-release of it among them, or a SNAPSHOT, a build's own published locally. The releases before
/// were never GitHub's, and are not served.
fn served(compiler: &str) -> bool {
    compiler.ends_with("-SNAPSHOT") || numbers_of(compiler).is_some_and(|n| n >= FIRST_RELEASE)
}

/// Whether a compiler is a release before `FIRST_RELEASE` by its numbers, a pre-release of one among
/// them: one the extension refuses by name, where a version of no release's shape is passed over.
fn before_first(compiler: &str) -> bool {
    !compiler.ends_with("-SNAPSHOT") && numbers_of(compiler).is_some_and(|n| n < FIRST_RELEASE)
}

/// A version's `<major>.<minor>.<patch>`, before any `-` suffix.
fn numbers_of(compiler: &str) -> Option<[u64; 3]> {
    let core = compiler.split_once('-').map_or(compiler, |(core, _)| core);
    let numbers: Option<Vec<u64>> = core.split('.').map(number).collect();
    match numbers.as_deref() {
        Some(&[major, minor, patch]) => Some([major, minor, patch]),
        _ => None,
    }
}

/// The digest of a file of the work directory, SHA-256 for a digest of 64 hex digits and SHA-1
/// otherwise, and its size.
fn digest_of_file(path: impl AsRef<Path>, hex_len: usize) -> std::io::Result<(String, u64)> {
    if hex_len != 64 {
        return sha1_of_file(path);
    }
    let mut file = fs::File::open(path)?;
    let mut sha256 = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sha256.update(&buf[..n]);
    }
    let size = sha256.length;
    Ok((sha256.hex(), size))
}

/// The sha1 of a file of the work directory and its size.
fn sha1_of_file(path: impl AsRef<Path>) -> std::io::Result<(String, u64)> {
    let mut file = fs::File::open(path)?;
    let mut sha1 = Sha1::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sha1.update(&buf[..n]);
    }
    let size = sha1.length;
    Ok((sha1.hex(), size))
}

/// SHA-1 (FIPS 180-4), as `src/task/fetch.rs` has it.
struct Sha1 {
    state: [u32; 5],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

impl Sha1 {
    fn new() -> Sha1 {
        Sha1 { state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], block: [0; 64], filled: 0, length: 0 }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.length += bytes.len() as u64;
        while !bytes.is_empty() {
            let take = (64 - self.filled).min(bytes.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.state;
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e]) {
            *s = s.wrapping_add(v);
        }
    }

    fn hex(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        self.state.iter().map(|w| format!("{:08x}", w)).collect()
    }
}

/// SHA-256 (FIPS 180-4).
struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Sha256 {
    fn new() -> Sha256 {
        Sha256 { state: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19], block: [0; 64], filled: 0, length: 0 }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        while !bytes.is_empty() {
            let take = (64 - self.filled).min(bytes.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }

    fn hex(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        self.state.iter().map(|w| format!("{:08x}", w)).collect()
    }
}

zed::register_extension!(TeqExtension);

#[cfg(test)]
mod tests {
    use super::*;

    const LATEST_ANSWER: &str = include_str!("../tests/release-latest.json");

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap_or_else(|| panic!("{text} is no release's version"))
    }

    #[test]
    fn versions_in_order() {
        let ordered = ["0.1.0-pre.1", "0.1.0-pre.2", "0.1.0-rc.1", "0.1.0", "0.1.1", "0.1.2", "0.1.10", "0.2.0-pre.1", "0.2.0", "1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta.2", "1.0.0-beta.11", "1.0.0"];
        for pair in ordered.windows(2) {
            assert!(version(pair[0]) < version(pair[1]), "{} before {}", pair[0], pair[1]);
        }
        assert_eq!(version("0.1.1"), version("0.01.1"));
        assert_eq!(version("0.2.0-pre.1").text, "0.2.0-pre.1");
        for text in ["0.1.0-SNAPSHOT", "0.1.0-pre.SNAPSHOT", "0.1.0-snapshot", "0.1", "0.1.1.1", "v0.1.1", "0.1.0-", "0.1.0-pre..1", "0.1.0-pre-1", "0.1.0-pre_1", "", "snapshot", "ff15f2278065734035de325534184128c33eeba1", "99999999999999999999.0.0"] {
            assert!(Version::parse(text).is_none(), "{text} is no release's version");
        }
    }

    /// GitHub's answer for its latest release (tests/release-latest.json): every classifier's binary by the
    /// release's asset name, `.exe` for Windows alone, its digest from `SHA256SUMS`, its size GitHub's, and
    /// its URL the asset's own.
    #[test]
    fn the_release_github_lists() {
        let latest: serde_json::Value = serde_json::from_str(LATEST_ANSWER).unwrap();
        let classifiers = ["osx-aarch_64", "osx-x86_64", "linux-x86_64", "linux-aarch_64", "windows-x86_64"];
        let sums: String = classifiers.iter().enumerate().map(|(n, c)| format!("{}  {}\n", n.to_string().repeat(64), asset_name("0.1.7", c))).collect();
        let file = |url: &str| -> Result<String> {
            assert_eq!(url, format!("{GITHUB_RELEASES}/v0.1.7/SHA256SUMS"));
            Ok(sums.clone())
        };
        let assets = latest["assets"].as_array().unwrap();
        for (n, c) in classifiers.iter().enumerate() {
            let newest = release_of(&latest, &file, c).unwrap().unwrap();
            let asset = assets.iter().find(|a| a["name"] == asset_name("0.1.7", c).as_str()).unwrap();
            assert_eq!(newest, Release { version: version("0.1.7"), digest: n.to_string().repeat(64), size: asset["size"].as_u64().unwrap() });
            assert_eq!(newest.url(c), asset["browser_download_url"].as_str().unwrap());
        }
        let newest = release_of(&latest, &file, "osx-aarch_64").unwrap().unwrap();
        assert_eq!(newest.url("osx-aarch_64"), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-osx-aarch_64");
        assert_eq!(newest.url("windows-x86_64"), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-windows-x86_64.exe");
        assert_eq!((newest.dir(), newest.path("osx-aarch_64")), ("teq-0.1.7".to_string(), "teq-0.1.7/teq".to_string()));
        assert_eq!(newest.path("windows-x86_64"), "teq-0.1.7/teq.exe");
        // A platform the release has no binary for: none.
        assert_eq!(release_of(&latest, &file, "linux-x86").unwrap(), None);
    }

    /// A GitHub release's JSON as the API gives it: its tag, draft and pre-release flags, and its assets.
    fn github(tag: &str, draft: bool, prerelease: bool, assets: &[(&str, u64)]) -> serde_json::Value {
        let assets: Vec<serde_json::Value> = assets.iter().map(|(name, size)| serde_json::json!({"name": name, "size": size, "browser_download_url": format!("{GITHUB_RELEASES}/{tag}/{name}")})).collect();
        serde_json::json!({"tag_name": tag, "name": "a title naming another version, 9.9.9", "draft": draft, "prerelease": prerelease, "assets": assets})
    }

    /// A GitHub release's asset names: `.exe` for Windows alone, as the extension's copies.
    #[test]
    fn the_asset_names() {
        for c in ["osx-aarch_64", "osx-x86_64", "linux-x86_64", "linux-aarch_64"] {
            assert_eq!(asset_name("0.1.7", c), format!("teq-0.1.7-{c}"));
            assert_eq!(binary_name(c), "teq");
        }
        assert_eq!(asset_name("0.1.7", "windows-x86_64"), "teq-0.1.7-windows-x86_64.exe");
        assert_eq!(binary_name("windows-x86_64"), "teq.exe");
        let release = |c: &str| Release { version: version("0.1.7"), digest: "a".repeat(64), size: 1 }.url(c);
        assert_eq!(release("linux-aarch_64"), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-linux-aarch_64");
        assert_eq!(release("windows-x86_64"), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-windows-x86_64.exe");
    }

    /// The search as a start makes it, through stand-ins for GitHub's API (its latest release and its
    /// listing) and the releases' SHA256SUMS, and what the start does with what it found.
    #[test]
    fn the_releases_from_github() {
        let sha = |n: char| n.to_string().repeat(64);
        let c = "osx-aarch_64";
        let full = |v: &str| vec![(format!("teq-{v}-{c}"), 7u64), ("SHA256SUMS".to_string(), 300u64), (format!("teq-{v}-binaries.txt"), 400u64)];
        fn as_ref(a: &[(String, u64)]) -> Vec<(&str, u64)> {
            a.iter().map(|(n, s)| (n.as_str(), *s)).collect()
        }
        let sums = |v: &str| format!("{}  teq-{v}-{c}\n{}  teq-{v}-linux-x86_64\n", sha('a'), sha('b'));
        // latest: GitHub's /releases/latest answer (None for a 404, Err for a failure); listing: its pages.
        let search = |latest: Option<Result<serde_json::Value>>, listing: Vec<serde_json::Value>, sums_of: &dyn Fn(&str) -> Result<String>| {
            let asked = std::cell::RefCell::new(Vec::new());
            let api = |url: &str| -> Result<String> {
                asked.borrow_mut().push(url.to_string());
                if url == format!("{GITHUB_API}/releases/latest") {
                    return match &latest {
                        None => Err("failed to fetch 'x': status code 404 Not Found".to_string()),
                        Some(answer) => answer.clone().map(|v| v.to_string()),
                    };
                }
                let page: usize = url.strip_prefix(&format!("{GITHUB_API}/releases?per_page=100&page=")).unwrap_or_else(|| panic!("{url} is no listing")).parse().unwrap();
                Ok(serde_json::Value::Array(listing.get(page - 1).and_then(|p| p.as_array()).cloned().unwrap_or_default()).to_string())
            };
            let file = |url: &str| -> Result<String> {
                asked.borrow_mut().push(url.to_string());
                let v = url.strip_prefix(&format!("{GITHUB_RELEASES}/v")).and_then(|r| r.strip_suffix("/SHA256SUMS")).unwrap_or_else(|| panic!("{url} is no SHA256SUMS"));
                sums_of(v)
            };
            let found = discover(&api, &file, c);
            (found, asked.into_inner())
        };
        let ok_sums = |v: &str| Ok(sums(v));
        // A stable release: it alone, its digest from SHA256SUMS, its URL the canonical asset's; no listing asked.
        let (found, asked) = search(Some(Ok(github("v0.1.8", false, false, &as_ref(&full("0.1.8"))))), vec![], &ok_sums);
        let found = found.unwrap();
        assert_eq!(found, vec![Release { version: version("0.1.8"), digest: sha('a'), size: 7 }]);
        assert_eq!(asked, vec![format!("{GITHUB_API}/releases/latest"), format!("{GITHUB_RELEASES}/v0.1.8/SHA256SUMS")]);
        assert_eq!(found[0].url(c), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.8/teq-0.1.8-osx-aarch_64");
        let absent = |_: &Release| false;
        assert_eq!(plan(Ok(found.clone()), &absent, &|| Some("teq-0.1.7/teq".to_string()), c), Plan::Fetch(found[0].clone()));
        // A stable release without the platform's binary, or without SHA256SUMS: none from GitHub, no
        // pre-release in its place.
        let no_binary = github("v0.1.8", false, false, &[("SHA256SUMS", 300), ("teq-0.1.8-linux-x86_64", 7)]);
        let prerelease = github("v0.1.9-rc.1", false, true, &as_ref(&full("0.1.9-rc.1")));
        let (found, asked) = search(Some(Ok(no_binary)), vec![serde_json::json!([prerelease])], &ok_sums);
        assert_eq!(found.unwrap(), vec![]);
        assert!(!asked.iter().any(|u| u.contains("releases?")), "no listing while a stable release exists: {asked:?}");
        let (found, _) = search(Some(Ok(github("v0.1.8", false, false, &[("teq-0.1.8-osx-aarch_64", 7)]))), vec![], &ok_sums);
        assert_eq!(found.unwrap(), vec![]);
        // The macOS binary under the name Windows's has: no asset of the platform.
        let (found, _) = search(Some(Ok(github("v0.1.8", false, false, &[("teq-0.1.8-osx-aarch_64.exe", 7), ("SHA256SUMS", 300)]))), vec![], &ok_sums);
        assert_eq!(found.unwrap(), vec![]);
        // No stable release: the newest non-draft pre-release by its tag's version, over two pages; a draft
        // never, an asset named for another version never.
        let page1: Vec<serde_json::Value> = (0..100).map(|n| github(&format!("v0.1.7-rc.{n}"), false, true, &as_ref(&full(&format!("0.1.7-rc.{n}"))))).collect();
        let page2 = vec![
            github("v0.2.0-rc.1", true, true, &as_ref(&full("0.2.0-rc.1"))),
            github("v0.1.9-rc.2", false, true, &as_ref(&full("0.1.9-rc.2"))),
            github("v0.1.9-rc.1", false, true, &as_ref(&full("0.1.9-rc.1"))),
        ];
        let (found, asked) = search(None, vec![serde_json::json!(page1), serde_json::json!(page2)], &ok_sums);
        assert_eq!(found.unwrap(), vec![Release { version: version("0.1.9-rc.2"), digest: sha('a'), size: 7 }]);
        assert_eq!(asked.iter().filter(|u| u.contains("releases?")).count(), 2);
        let mismatched = github("v0.1.9", false, true, &[("teq-0.1.8-osx-aarch_64", 7), ("SHA256SUMS", 300)]);
        let (found, _) = search(None, vec![serde_json::json!([mismatched])], &ok_sums);
        assert_eq!(found.unwrap(), vec![]);
        // Drafts alone: none from GitHub.
        let (found, _) = search(None, vec![serde_json::json!([github("v0.1.9", true, false, &as_ref(&full("0.1.9")))])], &ok_sums);
        assert_eq!(found.unwrap(), vec![]);
        // SHA256SUMS naming the binary twice, or not as a SHA-256, fails the search.
        let twice = |v: &str| Ok(format!("{}  teq-{v}-{c}\n{} *teq-{v}-{c}\n", sha('a'), sha('a')));
        assert!(search(Some(Ok(github("v0.1.8", false, false, &as_ref(&full("0.1.8"))))), vec![], &twice).0.is_err());
        let short = |v: &str| Ok(format!("abc  teq-{v}-{c}\n"));
        assert!(search(Some(Ok(github("v0.1.8", false, false, &as_ref(&full("0.1.8"))))), vec![], &short).0.is_err());
        // GitHub refusing more requests: a failure saying so, never "no release", never a pre-release.
        let (found, asked) = search(Some(Err("failed to fetch 'x': status code 403 Forbidden".to_string())), vec![], &ok_sums);
        assert!(found.as_ref().unwrap_err().contains("refuses more requests"), "{found:?}");
        assert_eq!(asked.len(), 1);
        assert!(search(Some(Err("failed to fetch 'x': status code 429 Too Many Requests".to_string())), vec![], &ok_sums).0.unwrap_err().contains("refuses more requests"));
        assert_eq!(plan(Err("GitHub refuses more requests".to_string()), &absent, &|| Some("teq-0.1.8/teq".to_string()), c), Plan::Serve("teq-0.1.8/teq".to_string()));
        // A release before 0.1.7, the latest or a pre-release, is none, its SHA256SUMS never asked.
        let (found, asked) = search(Some(Ok(github("v0.1.6", false, false, &as_ref(&full("0.1.6"))))), vec![], &ok_sums);
        assert_eq!((found.unwrap(), asked.iter().any(|u| u.contains("SHA256SUMS"))), (vec![], false));
        let (found, asked) = search(None, vec![serde_json::json!([github("v0.1.6-rc.1", false, true, &as_ref(&full("0.1.6-rc.1")))])], &ok_sums);
        assert_eq!((found.unwrap(), asked.iter().any(|u| u.contains("SHA256SUMS"))), (vec![], false));
    }

    /// What a search of GitHub found, kept between starts with its time.
    #[test]
    fn the_metadata_kept_between_starts() {
        let release = Release { version: version("0.1.8"), digest: "a".repeat(64), size: 7 };
        let text = cached_text(1_000, &release, "linux-x86_64");
        assert_eq!(cached_in(&text, "linux-x86_64"), Some((1_000, release.clone())));
        assert_eq!(cached_in(&text, "osx-aarch_64"), None);
        assert_eq!(cached_in("{not json", "linux-x86_64"), None);
        assert_eq!(cached_in(&text.replace(&"a".repeat(64), "abc"), "linux-x86_64"), None);
        // A search that found a release before 0.1.7 stands for nothing, fresh or not.
        let old = Release { version: version("0.1.6"), digest: "a".repeat(64), size: 7 };
        assert_eq!(cached_in(&cached_text(1_000, &old, "linux-x86_64"), "linux-x86_64"), None);
    }

    #[test]
    fn the_newest_copy_present() {
        let dir = std::env::temp_dir().join(format!("zed-teq-copies-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(newest_copy(&dir, "linux-x86_64"), None);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(newest_copy(&dir, "linux-x86_64"), None);
        let copy = |name: &str, bytes: usize, stamp: Option<&str>| {
            fs::create_dir_all(dir.join(name)).unwrap();
            fs::write(dir.join(name).join("teq"), vec![0u8; bytes]).unwrap();
            if let Some(stamp) = stamp {
                fs::write(dir.join(name).join("stamp"), stamp).unwrap();
            }
        };
        // The sha1 of 3, 5 and 9 zero bytes.
        let (sha1_3, sha1_5, sha1_9) = ("29e2dcfbb16f63bb0254df7585a15bb6fb5e927d", "a10909c2cdcaf5adb7e6b092a4faba558b62bd96", "c259e771b237769cb6bce9a5ab734c576a6da3e1");
        copy("teq-0.1.7", 3, Some(&format!("{sha1_3} 3\n")));
        copy("teq-0.1.8", 5, Some(&format!("{sha1_5} 5")));
        copy("teq-0.1.9", 5, Some(&format!("{sha1_5} 6")));
        copy("teq-0.1.10", 5, None);
        copy("teq-0.1.11", 5, Some("not a stamp"));
        // Its size the stamp's, its digest another's.
        copy("teq-0.1.12", 5, Some(&format!("{sha1_3} 5")));
        copy("teq-ff15f2278065734035de325534184128c33eeba1", 9, Some(&format!("{sha1_9} 9")));
        copy("teq-snapshot", 9, Some(&format!("{sha1_9} 9")));
        fs::create_dir_all(dir.join("teq-0.1.20")).unwrap();
        fs::write(dir.join("teq-0.1.20").join("teq.download"), b"part").unwrap();
        assert_eq!(newest_copy(&dir, "linux-x86_64").as_deref(), Some("teq-0.1.8/teq"));
        // On Windows the copy is teq.exe, and a bare teq is none.
        assert_eq!(newest_copy(&dir, "windows-x86_64"), None);
        fs::rename(dir.join("teq-0.1.8").join("teq"), dir.join("teq-0.1.8").join("teq.exe")).unwrap();
        assert_eq!(newest_copy(&dir, "windows-x86_64").as_deref(), Some("teq-0.1.8/teq.exe"));
        for gone in ["teq-0.1.9", "teq-0.1.10", "teq-0.1.11", "teq-0.1.12"] {
            assert!(!dir.join(gone).exists(), "{gone} stays");
        }
        for kept in ["teq-0.1.7", "teq-ff15f2278065734035de325534184128c33eeba1", "teq-snapshot", "teq-0.1.20"] {
            assert!(dir.join(kept).exists(), "{kept} is gone");
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The verified copies of releases before the first served, which an extension of before fetched:
    /// none serves, a newer copy found past them, and none is removed.
    #[test]
    fn no_copy_of_an_old_release_serves() {
        let dir = std::env::temp_dir().join(format!("zed-teq-old-copies-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let copy = |name: &str, bytes: usize, stamp: &str| {
            fs::create_dir_all(dir.join(name)).unwrap();
            fs::write(dir.join(name).join("teq"), vec![0u8; bytes]).unwrap();
            fs::write(dir.join(name).join("stamp"), stamp).unwrap();
        };
        // The sha1 of 3 and 5 zero bytes, and the SHA-256 of 5.
        let (sha1_3, sha1_5) = ("29e2dcfbb16f63bb0254df7585a15bb6fb5e927d", "a10909c2cdcaf5adb7e6b092a4faba558b62bd96");
        let sha256_5 = "8855508aade16ec573d21e6a485dfd0a7624085c1a14b5ecdd6485de0c6839a4";
        copy("teq-0.1.0-pre.1", 3, &format!("{sha1_3} 3\n"));
        copy("teq-0.1.6", 5, &format!("{sha1_5} 5\n"));
        copy("teq-0.1.6-rc.1", 5, &format!("{sha256_5} 5\n"));
        assert!(verified(&dir.join("teq-0.1.6/teq"), sha1_5, 5) && verified(&dir.join("teq-0.1.6-rc.1/teq"), sha256_5, 5));
        assert_eq!(newest_copy(&dir, "linux-x86_64"), None);
        let offline = || Err("GitHub refuses more requests for now (403)".to_string());
        let copies = || newest_copy(&dir, "linux-x86_64");
        assert_eq!(plan(offline(), &|_| false, &copies, "linux-x86_64"), Plan::Fail("the releases of teq could not be listed (GitHub refuses more requests for now (403))".to_string()));
        assert_eq!(plan(Ok(vec![]), &|_| false, &copies, "linux-x86_64"), Plan::Fail("the repository lists no release of teq for linux-x86_64".to_string()));
        copy("teq-0.1.7-rc.1", 5, &format!("{sha256_5} 5\n"));
        assert_eq!(newest_copy(&dir, "linux-x86_64").as_deref(), Some("teq-0.1.7-rc.1/teq"));
        for kept in ["teq-0.1.0-pre.1", "teq-0.1.6", "teq-0.1.6-rc.1"] {
            assert!(dir.join(kept).exists(), "{kept} is gone");
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_a_start_does() {
        let release = |text: &str, size: u64| Release { version: version(text), digest: "a".repeat(64), size };
        let found = || Ok(vec![release("0.1.7-rc.1", 1), release("0.1.9", 2), release("0.1.8", 3)]);
        let gone = || Err("timed out".to_string());
        let present = |r: &Release| r.version.text == "0.1.9" && r.size == 2;
        let absent = |_: &Release| false;
        let copy = || Some("teq-0.1.8/teq".to_string());
        let none = || None;
        let unasked = || panic!("the copies are asked for with a release found");
        assert_eq!(plan(found(), &present, &unasked, "c"), Plan::Reuse("teq-0.1.9/teq".to_string()));
        assert_eq!(plan(found(), &absent, &unasked, "c"), Plan::Fetch(release("0.1.9", 2)));
        assert_eq!(plan(Ok(vec![]), &present, &copy, "c"), Plan::Serve("teq-0.1.8/teq".to_string()));
        assert_eq!(plan(gone(), &present, &copy, "c"), Plan::Serve("teq-0.1.8/teq".to_string()));
        assert_eq!(plan(Ok(vec![]), &present, &none, "c"), Plan::Fail("the repository lists no release of teq for c".to_string()));
        assert_eq!(plan(gone(), &present, &none, "c"), Plan::Fail("the releases of teq could not be listed (timed out)".to_string()));
    }

    /// A copy whose bytes changed in place at the same size, under the metadata of a fresh search and
    /// with GitHub out of reach: neither reused nor served.
    #[test]
    fn a_copy_changed_at_the_same_size() {
        let dir = std::env::temp_dir().join(format!("zed-teq-changed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (right, wrong, older) = (b"#!/bin/sh\necho RIGHT\n", b"#!/bin/sh\necho WRONG\n", b"#!/bin/sh\necho OLDER\n");
        let right_sha256 = "e159495f47adb81aba27691104410cb84daffb6720331aa9ad4f1ac90b5b29d1";
        let older_sha256 = "0ca580a3e5a15d372463960485065baea2934a25a109ee543bc3da444283ca3f";
        let copy = |name: &str, bytes: &[u8], digest: &str| {
            fs::create_dir_all(dir.join(name)).unwrap();
            fs::write(dir.join(name).join("teq"), bytes).unwrap();
            fs::write(dir.join(name).join("stamp"), format!("{digest} {}\n", bytes.len())).unwrap();
        };
        let release = Release { version: version("0.1.8"), digest: right_sha256.to_string(), size: right.len() as u64 };
        // What fetched_binary asks of a copy, in the directory.
        let present = |r: &Release| verified(&dir.join(r.path("linux-x86_64")), &r.digest, r.size);
        let copies = || newest_copy(&dir, "linux-x86_64");
        let offline = || Err("GitHub refuses more requests for now (403)".to_string());
        copy("teq-0.1.8", right, right_sha256);
        assert!(present(&release));
        assert_eq!(plan(Ok(vec![release.clone()]), &present, &copies, "linux-x86_64"), Plan::Reuse("teq-0.1.8/teq".to_string()));
        assert_eq!(plan(offline(), &present, &copies, "linux-x86_64"), Plan::Serve("teq-0.1.8/teq".to_string()));

        fs::write(dir.join("teq-0.1.8/teq"), wrong).unwrap();
        assert_eq!(fs::metadata(dir.join("teq-0.1.8/teq")).unwrap().len(), release.size);
        // The fresh metadata stands only while its copy is the release's: here the search is made again, and
        // with GitHub out of reach the kept release is fetched, not reused.
        assert!(!present(&release));
        assert_eq!(plan(Ok(vec![release.clone()]), &present, &copies, "linux-x86_64"), Plan::Fetch(release.clone()));
        // With nothing found, the changed copy does not serve, and goes; an older copy that is its release's does.
        copy("teq-0.1.7", older, older_sha256);
        assert_eq!(plan(offline(), &present, &copies, "linux-x86_64"), Plan::Serve("teq-0.1.7/teq".to_string()));
        assert!(!dir.join("teq-0.1.8").exists());
        fs::write(dir.join("teq-0.1.7/teq"), wrong).unwrap();
        assert_eq!(plan(offline(), &present, &copies, "linux-x86_64"), Plan::Fail("the releases of teq could not be listed (GitHub refuses more requests for now (403))".to_string()));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_stamp_of_a_copy() {
        assert_eq!(stamp_in("D67C6E8D0000000000000000000000000000ABCD 19230880\n"), Some(("d67c6e8d0000000000000000000000000000abcd".to_string(), 19230880)));
        assert_eq!(stamp_in(&format!("{} 7", "A".repeat(64))), Some(("a".repeat(64), 7)));
        assert_eq!(stamp_in("d67c6e8d0000000000000000000000000000abcd"), None);
        assert_eq!(stamp_in("nope 5"), None);
        assert_eq!(stamp_in(""), None);
    }

    #[test]
    fn classifiers_by_platform() {
        assert_eq!(classifier_for((Os::Mac, Architecture::Aarch64)), "osx-aarch_64");
        assert_eq!(classifier_for((Os::Mac, Architecture::X8664)), "osx-x86_64");
        assert_eq!(classifier_for((Os::Linux, Architecture::X8664)), "linux-x86_64");
        assert_eq!(classifier_for((Os::Linux, Architecture::Aarch64)), "linux-aarch_64");
        assert_eq!(classifier_for((Os::Windows, Architecture::X8664)), "windows-x86_64");
    }

    #[test]
    fn the_binary_a_lock_pins() {
        let url = "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-osx-aarch_64";
        let lock = format!("teq: 0.1.7\nformat: 1\nbinaries:\n  osx-aarch_64: {url} FF15F2278065734035DE325534184128C33EEBA1 17334592\nprojects: {{}}\n");
        let pin = Pin { url: url.to_string(), sha1: "ff15f2278065734035de325534184128c33eeba1".to_string(), size: 17334592 };
        assert_eq!(pin_in(&lock, "osx-aarch_64"), Ok(Some(pin)));
        assert_eq!(pin_in(&lock, "linux-x86_64"), Ok(None));
        assert_eq!(pin_in("teq: 0.1.7\nformat: 1\nbinaries:\n  osx-aarch_64: https://r/teq ff15 1\n", "osx-aarch_64"), Ok(None));
        assert_eq!(pin_in(&lock.replace("format: 1", "format: 2"), "osx-aarch_64"), Ok(None));
        assert_eq!(pin_in("{not a lock", "osx-aarch_64"), Ok(None));
        // A lock of the release still unpinned, its table empty until the release's pin: none.
        assert_eq!(pin_in("teq: 0.1.7\nformat: 1\nbinaries: {}\nprojects: {}\n", "osx-aarch_64"), Ok(None));
        // A later release, a pre-release of the first served and a SNAPSHOT pin theirs.
        for compiler in ["0.1.8", "0.2.0", "1.0.0", "0.1.7-rc.1", "0.1.8-SNAPSHOT"] {
            assert!(pin_in(&lock.replace("teq: 0.1.7", &format!("teq: {compiler}")), "osx-aarch_64").is_ok_and(|p| p.is_some()), "{compiler}");
        }
    }

    /// A lock of a compiler before the first release served, its binaries at the Maven repository that
    /// held them: no pin, whatever its URLs, so that the PATH and the release's copy follow.
    #[test]
    fn no_lock_of_an_old_release_pins() {
        let old = |compiler: &str| {
            let url = format!("https://repository.example/maven-releases/build/teq/teq/{compiler}/teq-{compiler}-osx-aarch_64.exe");
            format!("teq: {compiler}\nformat: 1\nbinaries:\n  osx-aarch_64: {url} ff15f2278065734035de325534184128c33eeba1 17334592\nprojects: {{}}\n")
        };
        for compiler in ["0.1.6", "0.1.1", "0.1.0", "0.1.0-pre.1", "0.1.6-rc.1", "0.0.9"] {
            assert_eq!(pin_in(&old(compiler), "osx-aarch_64"), Err(compiler.to_string()), "{compiler}");
        }
        // A version of no release's shape pins nothing, passed over as before.
        for compiler in ["0.1", "latest"] {
            assert_eq!(pin_in(&old(compiler), "osx-aarch_64"), Ok(None), "{compiler}");
        }
        // The same lines under a served compiler pin: the refusal is the compiler's.
        assert!(matches!(pin_in(&old("0.1.7"), "osx-aarch_64"), Ok(Some(_))));
        for (compiler, ran) in [("0.1.6", false), ("0.1.7", true), ("0.1.7-rc.1", true), ("0.1.10", true), ("1.0.0", true), ("0.1.5-SNAPSHOT", true), ("v0.1.7", false), ("", false), ("latest", false)] {
            assert_eq!(served(compiler), ran, "{compiler}");
        }
    }

    /// The extension's reader of the lock is teq's own, `src/task/lock.rs`, copied.
    #[test]
    fn the_lock_reader_is_teqs() {
        assert!(include_str!("lock.rs") == include_str!("../../../src/task/lock.rs"), "integrations/zed/src/lock.rs differs from src/task/lock.rs: copy it again");
    }

    #[test]
    fn sha256_of_the_standard_vectors() {
        let sha256 = |bytes: &[u8]| {
            let mut s = Sha256::new();
            s.update(bytes);
            s.hex()
        };
        assert_eq!(sha256(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
        let mut long = Sha256::new();
        for _ in 0..1000 {
            long.update(&[b'a'; 1000]);
        }
        assert_eq!(long.hex(), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }

    #[test]
    fn sha1_of_the_standard_vectors() {
        let sha1 = |bytes: &[u8]| {
            let mut s = Sha1::new();
            s.update(bytes);
            s.hex()
        };
        assert_eq!(sha1(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
    }
}
