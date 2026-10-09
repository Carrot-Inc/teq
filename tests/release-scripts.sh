#!/bin/bash
# Checks of the release scripts' decisions without the network (docs/DEVELOPING.md, "Releases"): CHANGELOG.md's
# section as bench/changelog.sh reads it, bench/release.sh's refusal of a version without one and its two commits
# (the compiler's bump, the plugin's), and
# bench/github-release.sh's verbs (the draft, publish, abandon, check, the dry run): the push, the draft, its
# resumption, the publication and the read-back, the refusals and the exit codes. They run in a scratch repository
# of the scripts, the qualification file, the two files that name the compiler's version and the plugin's version
# line, NOTICE and LICENSE, its hub and its GitHub bare
# repositories (GitHub's URL rewritten to its path by git's insteadOf), gh a stand-in that keeps the releases in
# a state file, the release's and Central's roots a local HTTP server. The token is a made-up one, and neither an output, a
# trace nor an argument of gh may carry it. Every command is bounded; the script exits 1 on a failure. KEEP=1
# keeps its scratch directory.
cd "$(dirname "$0")/.." || exit 1
work=$(mktemp -d "${TMPDIR:-/tmp}/release-scripts.XXXXXX") || exit 1
server=
trap '[ -z "$server" ] || kill "$server" 2> /dev/null; [ -n "${KEEP:-}" ] || rm -rf "$work"' EXIT
pass=0
fail=0
check() {
  if [ "$2" = "$3" ]; then pass=$((pass + 1)); else echo "FAIL $1: '$2', where '$3' is expected"; fail=$((fail + 1)); fi
}
# has <name> <file> <text>: the file holds the text.
has() {
  if grep -qF -- "$3" "$2"; then pass=$((pass + 1)); else echo "FAIL $1: no '$3' in:"; sed 's/^/  /' "$2" | tail -15; fail=$((fail + 1)); fi
}

# The stand-in for gh: the calls bench/github-release.sh makes, over $STUB_DIR/state.json, with GitHub's rules
# as far as the script meets them (a published release by tag before a draft, drafts of one tag allowed, the
# tag made at the target when a draft is published, a public download only of a published release's asset).
# STUB_FAIL_UPLOAD=<name> leaves that asset broken and fails, as an upload cut off; STUB_RACE=1 makes another
# draft of the tag beside the one asked for, as another run would; STUB_PUBLISH_ON_READ=1 publishes a release
# read by its id before answering, and STUB_AFTER_DOWNLOAD (`drop <asset>`, `retarget <commit>`) changes a
# release after its assets are downloaded, as another clone's run would meanwhile; STUB_MOVE_HUB=<commit> moves
# the hub's master (STUB_HUB) there at the first listing of the releases, as a landing would;
# STUB_FAIL_LISTS_AFTER=<n> fails every listing of the releases after the first n with HTTP 503.
mkdir -p "$work/bin"
cat > "$work/bin/gh" << 'EOF'
#!/usr/bin/env python3
import hashlib, json, os, shutil, subprocess, sys
d = os.environ["STUB_DIR"]
os.makedirs(d, exist_ok=True)
with open(d + "/calls", "a") as log:
    log.write(" ".join(sys.argv[1:]) + "\n")
if os.environ.get("GH_TOKEN") != os.environ["STUB_TOKEN"]:
    sys.exit("HTTP 401: Bad credentials (https://api.github.com/)")
path = d + "/state.json"
state = json.load(open(path)) if os.path.exists(path) else {"next": 1, "releases": []}
def save():
    json.dump(state, open(path, "w"), indent=1)
args = sys.argv[1:]
def opt(name, default=None):
    for i, a in enumerate(args):
        if a == name and i + 1 < len(args):
            return args[i + 1]
        if a.startswith(name + "="):
            return args[i][len(name) + 1:]
    return default
def flag(name):
    for a in args:
        if a == name:
            return True
        if a.startswith(name + "="):
            return a[len(name) + 1:] == "true"
    return None
def find(tag):
    for r in state["releases"]:
        if r["tag_name"] == tag and not r["draft"]:
            return r
    for r in state["releases"]:
        if r["tag_name"] == tag:
            return r
    sys.exit("release not found")
def git(*a):
    return subprocess.run(["git", "--git-dir=" + os.environ["STUB_GITHUB"]] + list(a), capture_output=True, text=True)
def public(r):
    if not r["draft"]:
        os.makedirs("%s/downloads/%s" % (d, r["tag_name"]), exist_ok=True)
        for a in r["assets"]:
            shutil.copy("%s/files/%d/%s" % (d, r["id"], a["name"]), "%s/downloads/%s/%s" % (d, r["tag_name"], a["name"]))
def new(fields):
    r = {"id": state["next"], "tag_name": fields["tag_name"], "draft": True, "prerelease": fields["prerelease"],
         "target_commitish": fields["target_commitish"], "name": fields["name"], "body": fields["body"], "assets": []}
    state["next"] += 1
    state["releases"].append(r)
    return r
if args[0] == "api":
    method, fields, jq, end, i = "GET", {}, None, None, 1
    while i < len(args):
        a = args[i]
        if a == "-X":
            method, i = args[i + 1], i + 2
        elif a in ("-f", "-F"):
            k, v = args[i + 1].split("=", 1)
            if a == "-F":
                v = open(v[1:]).read() if v.startswith("@") else {"true": True, "false": False}.get(v, v)
            fields[k], i = v, i + 2
        elif a == "--jq":
            jq, i = args[i + 1], i + 2
        elif a == "--paginate":
            i += 1
        else:
            end, i = a, i + 1
    if end == "repos/Carrot-Inc/teq":
        print(json.dumps({"full_name": "Carrot-Inc/teq"}))
    elif method == "GET" and end.startswith("repos/Carrot-Inc/teq/releases?"):
        every = sorted(state["releases"], key=lambda r: -r["id"])
        if os.environ.get("STUB_FAIL_LISTS_AFTER"):
            lists = int(open(d + "/lists").read()) if os.path.exists(d + "/lists") else 0
            open(d + "/lists", "w").write(str(lists + 1))
            if lists >= int(os.environ["STUB_FAIL_LISTS_AFTER"]):
                sys.exit("HTTP 503: unavailable")
        if os.environ.get("STUB_MOVE_HUB") and not os.path.exists(d + "/hub-moved"):
            open(d + "/hub-moved", "w").close()
            subprocess.run(["git", "--git-dir=" + os.environ["STUB_HUB"], "update-ref", "refs/heads/master", os.environ["STUB_MOVE_HUB"]], check=True)
        # Pages of two, as --paginate prints them: one array after another.
        print("".join(json.dumps(every[i:i + 2], indent=1) for i in range(0, max(len(every), 1), 2)))
    elif method == "POST" and end == "repos/Carrot-Inc/teq/releases":
        assert fields["draft"] is True and jq == ".id", (fields, jq)
        r = new(fields)
        if os.environ.get("STUB_RACE"):
            new(fields)
        save()
        print(r["id"])
    elif method == "GET" and end.startswith("repos/Carrot-Inc/teq/releases/") and end.rsplit("/", 1)[1].isdigit():
        r = [r for r in state["releases"] if r["id"] == int(end.rsplit("/", 1)[1])][0]
        if os.environ.get("STUB_PUBLISH_ON_READ"):
            r["draft"] = False
            save()
        assert jq == ".draft", jq
        print("true" if r["draft"] else "false")
    elif method == "DELETE" and end.startswith("repos/Carrot-Inc/teq/releases/"):
        gone = int(end.rsplit("/", 1)[1])
        state["releases"] = [r for r in state["releases"] if r["id"] != gone]
        shutil.rmtree("%s/files/%d" % (d, gone), ignore_errors=True)
        save()
    else:
        sys.exit("stub: no %s %s" % (method, end))
    sys.exit(0)
assert args[0] == "release" and opt("--repo") == "Carrot-Inc/teq", args
verb, tag = args[1], args[2]
if verb == "edit":
    r = find(tag)
    if opt("--title") is not None:
        r["name"] = opt("--title")
    if opt("--notes-file") is not None:
        r["body"] = open(opt("--notes-file")).read()
    if flag("--prerelease") is not None:
        r["prerelease"] = flag("--prerelease")
    if opt("--target") is not None:
        r["target_commitish"] = opt("--target")
    if flag("--latest") is not None:
        r["make_latest"] = flag("--latest")
    if flag("--draft") is False and r["draft"]:
        if git("rev-parse", "--verify", "--quiet", "refs/tags/" + tag).returncode != 0:
            if git("update-ref", "refs/tags/" + tag, r["target_commitish"]).returncode != 0:
                sys.exit("HTTP 422: target_commitish is invalid")
        r["draft"] = False
        public(r)
    save()
elif verb == "upload":
    r = find(tag)
    os.makedirs("%s/files/%d" % (d, r["id"]), exist_ok=True)
    for f in [a for a in args[3:] if not a.startswith("--") and a != "Carrot-Inc/teq"]:
        name = os.path.basename(f)
        if [a for a in r["assets"] if a["name"] == name] and not flag("--clobber"):
            sys.exit("asset under the same name already exists: " + name)
        r["assets"] = [a for a in r["assets"] if a["name"] != name]
        data = open(f, "rb").read()
        if os.environ.get("STUB_FAIL_UPLOAD") == name:
            open("%s/files/%d/%s" % (d, r["id"], name), "wb").write(data[:3])
            r["assets"].append({"name": name, "size": 3, "state": "starter", "digest": None})
            save()
            sys.exit("upload of %s failed: connection reset" % name)
        open("%s/files/%d/%s" % (d, r["id"], name), "wb").write(data)
        r["assets"].append({"name": name, "size": len(data), "state": "uploaded", "digest": "sha256:" + hashlib.sha256(data).hexdigest()})
    save()
    public(r)
elif verb == "download":
    r = find(tag)
    for a in r["assets"]:
        if a["state"] != "uploaded":
            sys.exit("asset %s is not uploaded" % a["name"])
        shutil.copy("%s/files/%d/%s" % (d, r["id"], a["name"]), os.path.join(opt("--dir"), a["name"]))
    hook = os.environ.get("STUB_AFTER_DOWNLOAD", "")
    if hook.startswith("drop "):
        r["assets"] = [a for a in r["assets"] if a["name"] != hook[5:]]
    elif hook.startswith("retarget "):
        r["target_commitish"] = hook[9:]
    save()
else:
    sys.exit("stub: no release " + verb)
EOF
chmod +x "$work/bin/gh"

# The roots the scripts ask: a local HTTP server over $work/www, answering 404 for what it does not hold.
mkdir -p "$work/www"
port=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
python3 -m http.server --bind 127.0.0.1 --directory "$work/www" "$port" > /dev/null 2>&1 &
server=$!
for _ in $(seq 50); do curl -s -o /dev/null --max-time 1 "http://127.0.0.1:$port/" && break; sleep 0.1; done

# The scratch repository: the scripts under test, the qualification file, the two files at 0.1.6, the plugin's
# version line at 1.0.0, a changelog without 0.1.7, NOTICE and LICENSE.
src=$work/src
mkdir -p "$src/bench" "$src/src" "$src/integrations/sbt"
cp bench/release.sh bench/ship-release.sh bench/ship-manifest.sh bench/changelog.sh bench/github-release.sh bench/ship-profiles.sh bench/ship-qualified.txt "$src/bench/"
cp NOTICE LICENSE "$src/"
printf '[package]\nname = "teq"\nversion = "0.1.6"\nedition = "2021"\n' > "$src/Cargo.toml"
printf 'fn main() {}\n' > "$src/src/main.rs"
printf '1.0.0\n' > "$src/integrations/sbt/plugin-version.txt"
printf '# Changelog\n\n## 0.1.6 and before\n\n- Everything until now.\n' > "$src/CHANGELOG.md"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=$work/gitconfig
git config --global user.name test && git config --global user.email test@example.invalid && git config --global init.defaultBranch master
# A helper that would store a credential: the script's own must set it aside.
git config --global credential.helper "store --file=$work/stored-credentials"
git config --global url."$work/github.git".insteadOf https://github.com/Carrot-Inc/teq.git
(cd "$src" && timeout 120 cargo generate-lockfile --offline -q && git init -q && git add -A && git commit -qm Initial) || { echo "FAIL the scratch repository"; exit 1; }
git clone -q --bare "$src" "$work/hub.git" && git clone -q --bare "$src" "$work/github.git" && git clone -q "$work/hub.git" "$work/repo" || exit 1
repo=$work/repo
# The private names, as the scripts read them; the GitHub release's and Maven Central's roots the server, which
# serves nothing there.
export TEQ_PRIVATE_NAMES=$work/private-names
export TEQ_RELEASES_BASE=http://127.0.0.1:$port/releases/download TEQ_CENTRAL_ROOT=http://127.0.0.1:$port/maven2
printf '# made-up names of this check\nZorblax\nSecret  App\n' > "$work/private-names"

# changelog_notes on its own.
. bench/changelog.sh
cl=$work/changelog.md
printf '# C\n\n## 0.1.8 (2026-10-09)\n\n\n- One,\n  two.\n- Three.\n\n\n## 0.1.7 (2026-10-08)\n\n- Older.\n## 0.1.70 (2026-10-01)\n- Not this.\n' > "$cl"
check "a section's text, its blank lines at either end dropped" "$(changelog_notes "$cl" 0.1.8)" "$(printf -- '- One,\n  two.\n- Three.')"
check "a section below the newest" "$(changelog_notes "$cl" 0.1.7 2>&1)" "$cl's section for 0.1.7 is below '## 0.1.8 (2026-10-09)': the newest comes first"
printf '## 0.1.7 (2026-10-08)\n\n- This.\n\n## 0.1.70 (2026-10-01)\n\n- Not this.\n' > "$cl"
check "a version is a field, not a prefix" "$(changelog_notes "$cl" 0.1.7 2>&1)" "- This."
check "no section" "$(changelog_notes "$cl" 0.1.9 2>&1)" "$cl has no section for 0.1.9: '## 0.1.9 (<yyyy-mm-dd>)' and its bullets, the newest first"
printf '## 0.1.8\n\n- One.\n' > "$cl"
check "a heading without its date" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's heading '## 0.1.8' is not '## 0.1.8 (<yyyy-mm-dd>)'"
printf '## 0.1.8 (2026-10-09)\n\nNothing.\n\n## 0.1.7 (2026-10-08)\n\n- Older.\n' > "$cl"
check "a section without a bullet" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 has no bullet"
printf '## 0.1.8 (2026-10-09)\n\n- \n-\n\n## 0.1.7 (2026-10-08)\n\n- Older.\n' > "$cl"
check "a section whose bullets are empty" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 has no bullet"
printf '## 0.1.8 (2026-10-09)\n\n- One.\n\n## 0.1.8 (2026-10-08)\n\n- Two.\n' > "$cl"
check "two sections" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl has 2 sections for 0.1.8"
printf '## 0.1.8 (2026-10-09)\n\n- Found by the Codex pass at the gate, see docs/internal/TARGETS.md.\n' > "$cl"
check "the process's words and an internal file" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 names the work's process or its internal files: Codex gate pass docs/internal "
printf '## 0.1.8 (2026-10-09)\n\n- What zorblax'"'"'s tests found; agentive and propagated are words of their own.\n' > "$cl"
check "a private name, case aside" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 names what $work/private-names keeps private: zorblax "
printf '## 0.1.8 (2026-10-09)\n\n- Fix Secret \n  App builds.\n' > "$cl"
check "a private name of two words wrapped over two lines" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 names what $work/private-names keeps private: Secret App "
check "the lines joined as the notes show them" "$(printf -- '- Fix Secret \n  App builds.\n- One.\n' | changelog_join)" "$(printf -- '- Fix Secret App builds.\n- One.')"
printf '## 0.1.8 (2026-10-09)\n\n- Fix the compiler after a review pass.\n' > "$cl"
check "the word pass" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 names the work's process or its internal files: pass "
printf '## 0.1.8 (2026-10-09)\n\n- An argument passed where a function is expected; a bypass and passes\n  of the typer.\n' > "$cl"
check "passed and bypass are no pass; passes is" "$(changelog_notes "$cl" 0.1.8 2>&1)" "$cl's section for 0.1.8 names the work's process or its internal files: passes "

# bench/release.sh: refused without the section, its checkout untouched; with it, the bump and the notes.
# release_case <name> <changelog text> <expected line>: the changelog committed, release.sh 0.1.7 refusing it.
release_case() {
  printf '%b' "$2" > "$repo/CHANGELOG.md"
  (cd "$repo" && git commit -q --allow-empty -am "Note: $1")
  (cd "$repo" && timeout 300 bench/release.sh 0.1.7 > "$work/release.out" 2>&1)
  check "release.sh refuses $1" "$?" 1
  has "release.sh refuses $1" "$work/release.out" "$3"
  check "release.sh leaves the checkout as it was: $1" "$(cd "$repo" && git status --porcelain)" ""
  (cd "$repo" && git reset -q --hard HEAD~1)
}
release_case "no section" '# Changelog\n\n## 0.1.6 and before\n\n- Everything until now.\n' "CHANGELOG.md has no section for 0.1.7"
release_case "an empty section" '# Changelog\n\n## 0.1.7 (2026-10-08)\n\n## 0.1.6 and before\n\n- Everything.\n' "CHANGELOG.md's section for 0.1.7 has no bullet"
release_case "an empty bullet" '# Changelog\n\n## 0.1.7 (2026-10-08)\n\n- \n\n## 0.1.6 and before\n\n- Everything.\n' "CHANGELOG.md's section for 0.1.7 has no bullet"
release_case "a section below the newest" '# Changelog\n\n## 0.1.6 and before\n\n- Everything.\n\n## 0.1.7 (2026-10-08)\n\n- A change.\n' "the newest comes first"
release_case "a private name" '# Changelog\n\n## 0.1.7 (2026-10-08)\n\n- What Zorblax needed.\n\n## 0.1.6 and before\n\n- Everything.\n' "keeps private: Zorblax"
printf '# Changelog\n\n## 0.1.7 (2026-10-08)\n\n- A change for users,\n  over two lines.\n- Another.\n\n## 0.1.6 and before\n\n- Everything until now.\n' > "$repo/CHANGELOG.md"
(cd "$repo" && git commit -qam "Note 0.1.7's changes" && timeout 300 bench/release.sh 0.1.7 > "$work/release.out" 2>&1)
check "release.sh takes a version with its section" "$?" 0
has "release.sh prints the notes" "$work/release.out" "  - A change for users,"
has "release.sh prints the commit" "$work/release.out" "git commit -m 'Release 0.1.7' Cargo.toml Cargo.lock"
check "release.sh bumps the compiler's two files alone" "$(cd "$repo" && git status --porcelain | sort | tr '\n' ' ')" " M Cargo.lock  M Cargo.toml "
(cd "$repo" && git commit -qm "Release 0.1.7" Cargo.toml Cargo.lock && git push -q origin master) || exit 1
# The plugin's bump, its own commit: a version after the line's, none Central serves.
(cd "$repo" && timeout 300 bench/release.sh --plugin 1.0.1 > "$work/release-plugin.out" 2>&1)
check "release.sh --plugin takes the next version" "$?" 0
has "release.sh --plugin prints its commit" "$work/release-plugin.out" "git commit -m 'Release sbt-teq 1.0.1' integrations/sbt/plugin-version.txt"
check "release.sh --plugin moves the plugin's line alone" "$(cd "$repo" && git status --porcelain)" " M integrations/sbt/plugin-version.txt"
(cd "$repo" && git checkout -q -- integrations/sbt/plugin-version.txt)
(cd "$repo" && timeout 300 bench/release.sh --plugin 1.0.0 > "$work/release-plugin.out" 2>&1)
check "release.sh --plugin refuses a version not after the line's" "$?" 1
has "release.sh --plugin refuses a version not after the line's" "$work/release-plugin.out" "1.0.0 does not come after the checkout's plugin 1.0.0"
release=$(cd "$repo" && git rev-parse HEAD)
initial=$(cd "$repo" && git rev-parse HEAD~2)
# The profiles that guided the binaries (bench/ship-profiles.sh), as the stage writes them from the ship's tree: a
# tree of the release's commit holding two trainings' products, whose asset is teq-0.1.7-profiles.tar.
profiles_asset() {
  local t=$work/ptree p
  rm -rf "$t" && git clone -q "$work/hub.git" "$t" && git -C "$t" checkout -q "$release" && mkdir -p "$t/target/pgo/ship" "$t/target/cross" || return 1
  echo 'c  a.scala' > "$t/target/pgo/ship/corpus.txt" && echo 'j  a.jar' > "$t/target/pgo/ship/jars.txt"
  for p in target/pgo/ship/teq target/cross/arm; do
    echo "counts of $p" > "$t/$p.profdata"
    printf 'training status ok\ntraining programs 2\ntraining runs 3\ntraining corpus %s 1 files\ntraining jars %s 1 jars\ntraining profile %s\ntraining metadata 0123abcd\ntraining release 0.1.7 %s\ntraining trainer t native\ntraining host h\ntraining tuple rustc 1.98.1 c\n' \
      "$(sha256sum < "$t/target/pgo/ship/corpus.txt" | cut -c1-64)" "$(sha256sum < "$t/target/pgo/ship/jars.txt" | cut -c1-64)" "$(sha256sum < "$t/$p.profdata" | cut -c1-64)" "$release" > "$t/$p.training"
  done
  mv "$t/target/pgo/ship/teq.training" "$t/target/pgo/ship/training.txt"
  timeout 60 bench/ship-profiles.sh write "$t" "$work/profiles" > /dev/null
}
profiles_asset || { echo "FAIL the profiles asset"; exit 1; }

# The staged set: the five classifiers, each with the manifest the ship writes (the toolchain the qualification
# file names, the Windows smoke and the aarch64 suite under its tools), staged by one invocation.
classifiers="osx-aarch_64 osx-x86_64 linux-x86_64 linux-aarch_64 windows-x86_64"
# asset_of <classifier>: the asset's name, .exe for the Windows one alone (bench/github-release.sh's rule).
asset_of() { case $1 in windows-*) echo "teq-0.1.7-$1.exe" ;; *) echo "teq-0.1.7-$1" ;; esac; }
binaries=$repo/integrations/sbt/binary/binaries
qualified=$repo/bench/ship-qualified.txt
stage() {
  local c sha
  rm -rf "$binaries" "$repo/out/ship/profiles"
  mkdir -p "$repo/out/ship/profiles" && cp "$work/profiles/teq-0.1.7-profiles.tar" "$repo/out/ship/profiles/"
  for c in $classifiers; do
    mkdir -p "$binaries/$c"
    head -c 20000 /dev/urandom > "$binaries/$c/teq"
    sha=$(sha256sum < "$binaries/$c/teq" | cut -c1-64)
    {
      printf 'binary %s\nversion teq 0.1.7 %s pgo\ncommit %s\nrun ship-test-1\n' "$sha" "${release:0:9}" "$release"
      # The profile that guided it, the asset's (the Windows binary is plain).
      case $c in osx-aarch_64 | linux-aarch_64) arch=aarch64 ;; windows-*) arch= ;; *) arch=x86_64 ;; esac
      [ -z "$arch" ] || tar -xOf "$work/profiles/teq-0.1.7-profiles.tar" profiles.txt |
        sed -n "s/^profile $arch \([0-9a-f]*\) .*/training profile \1/p; s/^metadata $arch /training metadata /p"
      sed -n '/^#/d; /^route /d; /^[a-z]/s/^/tuple /p' "$qualified"
      case $c in
        windows-x86_64) echo "smoke $sha tests/wine.sh passed: 1 of 1 under $(sed -n 's/^wine //p' "$qualified")" ;;
        linux-aarch_64) echo "suite $sha tests/run.sh passed: 1 passed, 0 failed, 0 of them skipped for missing jars, outputs 1 of 1 as the native suite's, under qemu-user $(sed -n 's/^qemu-user //p' "$qualified") sysroot $(sed -n 's/^sysroot-aarch64 //p' "$qualified")" ;;
      esac
    } > "$binaries/$c/teq.manifest"
  done
}
names="teq-0.1.7-osx-aarch_64 teq-0.1.7-osx-x86_64 teq-0.1.7-linux-x86_64 teq-0.1.7-linux-aarch_64 teq-0.1.7-windows-x86_64.exe SHA256SUMS teq-0.1.7-binaries.txt teq-0.1.7-profiles.tar NOTICE LICENSE"
token=ghp_madeUpForTheCheck0123456789abcdef
mkdir -p "$work/home/.config/teq"
printf '%s\n' "$token" > "$work/home/.config/teq/github-token"
chmod 600 "$work/home/.config/teq/github-token"
: > "$work/outputs"
# github <label> [args]: bench/github-release.sh in the checkout (under bash -x with XTRACE=1; in a GitHub Actions
# run with ACTIONS=1; GH_TOKEN set to ENVTOKEN; HOME=GHOME), its exit status echoed, its output in
# $work/out.<label> and among every output.
github() {
  local label=$1
  shift
  (cd "$repo" && env -u GH_TOKEN -u GITHUB_TOKEN -u GITHUB_ACTIONS HOME="${GHOME:-$work/home}" PATH="$work/bin:$PATH" STUB_DIR="$work/stub" \
    STUB_TOKEN="$token" STUB_GITHUB="$work/github.git" TEQ_GITHUB_DOWNLOADS="file://$work/stub/downloads" ${ACTIONS:+GITHUB_ACTIONS=true} \
    ${ENVTOKEN:+GH_TOKEN=$ENVTOKEN} timeout 300 ${XTRACE:+bash -x} bench/github-release.sh "$@" > "$work/out.$label" 2>&1)
  local status=$?
  cat "$work/out.$label" >> "$work/outputs"
  echo $status
}
# The stand-in's state: the releases' tags and states, its calls that begin so.
releases_now() { python3 -c 'import json, sys; s = json.load(open(sys.argv[1])); print(" ".join("%s:%s" % (r["tag_name"], "draft" if r["draft"] else "published") for r in s["releases"]))' "$work/stub/state.json" 2> /dev/null; }
calls() { grep -c "^$1" "$work/stub/calls" 2> /dev/null; }
github_ref() { git --git-dir="$work/github.git" rev-parse --verify --quiet "$1"; }
fresh() {
  rm -rf "$work/stub"
  mkdir -p "$work/stub"
  git --git-dir="$work/github.git" update-ref refs/heads/master "$1"
  git --git-dir="$work/github.git" tag -l | xargs -r git --git-dir="$work/github.git" tag -d > /dev/null
}

# The usage, and check: nothing written, the push a fast-forward.
check "no version: the usage" "$(github usage)" 2
check "an unknown verb: the usage" "$(github usage-verb 0.1.7 make)" 2
stage
fresh "$initial"
check "check passes" "$(github check 0.1.7 check)" 0
has "check: the push" "$work/out.check" "a fast-forward, which GitHub takes (git push --dry-run)"
has "check: no release yet" "$work/out.check" "none on GitHub yet"
check "check pushes nothing" "$(github_ref refs/heads/master)" "$initial"
check "check makes no release" "$(releases_now)" ""
check "publish with no draft" "$(github publish-none 0.1.7 publish)" 1
has "publish with no draft: why" "$work/out.publish-none" "GitHub has no release v0.1.7: bench/github-release.sh 0.1.7 makes its draft"

# The draft: GitHub's master pushed, a draft with every asset, no tag yet.
check "the draft" "$(github draft 0.1.7)" 0
check "the draft: GitHub's master is the hub's" "$(github_ref refs/heads/master)" "$release"
check "the draft: a draft" "$(releases_now)" "v0.1.7:draft"
check "the draft: no tag yet" "$(github_ref refs/tags/v0.1.7)" ""
has "the draft: what comes next" "$work/out.draft" "holds every asset: bench/github-release.sh 0.1.7 publish publishes it"
check "the draft run again: nothing written" "$(github draft-again 0.1.7)" 0
check "the draft run again: made once" "$(calls 'api -X POST')" 1
check "the draft run again: uploaded once each" "$(calls 'release upload')" 10
check "the draft run again: never edited" "$(calls 'release edit')" 0

# The publication: the tag on the release's commit, every asset read back from its URL.
check "publish" "$(github publish 0.1.7 publish)" 0
check "published: one published release" "$(releases_now)" "v0.1.7:published"
check "published: the tag on the release's commit" "$(github_ref 'refs/tags/v0.1.7^{commit}')" "$release"
has "published: read back" "$work/out.publish" "read back from file://$work/stub/downloads/v0.1.7/: $names, each the file, SHA256SUMS and teq-0.1.7-binaries.txt theirs"
python3 - "$work/stub/state.json" > "$work/release.json" << 'EOF'
import json, sys
r = json.load(open(sys.argv[1]))["releases"][0]
print(json.dumps({k: r.get(k) for k in ["name", "prerelease", "target_commitish", "make_latest"]}, sort_keys=True))
print(" ".join(a["name"] for a in r["assets"]))
print(r["body"], end="")
EOF
check "the release's title, flags and target" "$(sed -n 1p "$work/release.json")" "{\"make_latest\": true, \"name\": \"teq 0.1.7\", \"prerelease\": false, \"target_commitish\": \"$release\"}"
check "the assets, in their order" "$(sed -n 2p "$work/release.json")" "$names"
check "the notes: the section, a bullet's lines joined" "$(sed -n '3,$p' "$work/release.json")" "$(printf -- '- A change for users, over two lines.\n- Another.')"
downloads=$work/stub/downloads/v0.1.7
for c in $classifiers; do
  check "the asset of $c is its binary, uncompressed" "$(cmp "$downloads/$(asset_of $c)" "$binaries/$c/teq" && echo same)" same
done
check "SHA256SUMS: sha256sum's lines over the five, in their order" "$(cat "$downloads/SHA256SUMS")" \
  "$(cd "$binaries" && for c in $classifiers; do echo "$(sha256sum < "$c/teq" | cut -c1-64)  $(asset_of $c)"; done)"
(cd "$downloads" && sha256sum -c --quiet SHA256SUMS > /dev/null 2>&1)
check "SHA256SUMS checks the downloaded assets" "$?" 0
check "NOTICE and LICENSE, the release's commit's" "$(cmp "$downloads/NOTICE" NOTICE && cmp "$downloads/LICENSE" LICENSE && echo same)" same
check "the binaries' manifest: the version and the commit, then the classifier, the asset, sha256, sha1 and size" "$(cat "$downloads/teq-0.1.7-binaries.txt")" \
  "$(echo "teq 0.1.7 $release"; cd "$binaries" && for c in $classifiers; do echo "$c $(asset_of $c) $(sha256sum < "$c/teq" | cut -c1-64) $(sha1sum < "$c/teq" | cut -c1-40) $(wc -c < "$c/teq" | tr -d ' ')"; done)"
check "publish run again: read back again" "$(github publish-again 0.1.7 publish)" 0
check "publish run again: edited once, by the publication" "$(calls 'release edit')" 1
check "the draft run on a published release: checked alone" "$(github draft-published 0.1.7)" 0
has "the draft run on a published release: why" "$work/out.draft-published" "v0.1.7 is published already, its assets the files"
check "the draft run on a published release: nothing written" "$(calls 'release upload') $(calls 'release edit') $(calls 'api -X')" "10 1 1"
printf 'tampered\n' > "$downloads/teq-0.1.7-binaries.txt"
check "publish with an asset its URL does not serve" "$(github publish-tampered 0.1.7 publish)" 1
has "publish with an asset its URL does not serve: which" "$work/out.publish-tampered" "v0.1.7/teq-0.1.7-binaries.txt does not serve the file"
check "abandon of a published release" "$(github abandon-published 0.1.7 abandon)" 1
has "abandon of a published release: why" "$work/out.abandon-published" "a published release is never deleted"
check "abandon of a published release: kept" "$(releases_now)" "v0.1.7:published"

# A failed upload: exit 3, then the draft resumed, uploading what is missing alone; publish refuses the draft
# while it is not whole.
fresh "$initial"
check "an upload cut off fails once something is written" "$(STUB_FAIL_UPLOAD=teq-0.1.7-linux-x86_64 github cut 0.1.7)" 3
has "an upload cut off: the remedy" "$work/out.cut" "the same command, run again, resumes"
check "an upload cut off: the draft stays" "$(releases_now)" "v0.1.7:draft"
check "publish of a draft not whole" "$(github publish-cut 0.1.7 publish)" 1
has "publish of a draft not whole: why" "$work/out.publish-cut" "the draft v0.1.7 is not whole (teq-0.1.7-linux-x86_64 is not the file): bench/github-release.sh 0.1.7 resumes it"
check "publish of a draft not whole: still a draft, no tag" "$(releases_now) $(github_ref refs/tags/v0.1.7)" "v0.1.7:draft "
check "resumed" "$(github resumed 0.1.7)" 0
check "resumed: one draft" "$(releases_now)" "v0.1.7:draft"
check "resumed: made once" "$(calls 'api -X POST')" 1
check "resumed: the cut asset and the ones after it uploaded" "$(calls 'release upload')" 11
check "resumed, then published" "$(github resumed-publish 0.1.7 publish)" 0
check "resumed, then published: the tag" "$(github_ref 'refs/tags/v0.1.7^{commit}')" "$release"

# The ship gives the release up: its draft deleted; with none, nothing to do.
fresh "$initial"
check "a draft to abandon" "$(github abandoned-draft 0.1.7)" 0
check "abandon" "$(github abandon 0.1.7 abandon)" 0
check "abandon: no release left" "$(releases_now)" ""
check "abandon: no tag" "$(github_ref refs/tags/v0.1.7)" ""
check "abandon with nothing to delete" "$(github abandon-none 0.1.7 abandon)" 0
has "abandon with nothing to delete: why" "$work/out.abandon-none" "no draft of v0.1.7 to delete"
printf '%s\n' '{"next": 3, "releases": [{"id": 1, "tag_name": "v0.1.7", "draft": true, "prerelease": false, "target_commitish": "x", "name": "a", "body": "", "assets": []}, {"id": 2, "tag_name": "v0.1.7", "draft": true, "prerelease": false, "target_commitish": "x", "name": "b", "body": "", "assets": []}]}' > "$work/stub/state.json"
check "abandon of two drafts" "$(github abandon-two 0.1.7 abandon)" 0
check "abandon of two drafts: none left" "$(releases_now)" ""
# A listing that fails after the deletion is no absence: exit 3 after a deletion, 1 with nothing deleted.
check "a draft to abandon, its confirmation failing" "$(github abandoned-unconfirmed-draft 0.1.7)" 0
rm -f "$work/stub/lists"
check "abandon whose confirming listing fails" "$(STUB_FAIL_LISTS_AFTER=1 github abandon-unconfirmed 0.1.7 abandon)" 3
has "abandon whose confirming listing fails: why" "$work/out.abandon-unconfirmed" "GitHub's releases could not be read after the deletion"
rm -f "$work/stub/lists"
check "abandon of nothing whose confirming listing fails" "$(STUB_FAIL_LISTS_AFTER=1 github abandon-none-unconfirmed 0.1.7 abandon)" 1
has "abandon of nothing whose confirming listing fails: no absence claimed" "$work/out.abandon-none-unconfirmed" "GitHub's releases could not be read after the deletion"

# Two runs making a draft at once: the run whose draft is the second deletes it and fails; the next resumes.
fresh "$initial"
check "a draft made beside another run's" "$(STUB_RACE=1 github race 0.1.7)" 3
has "a draft made beside another run's: deleted" "$work/out.race" "another run made a release of v0.1.7 beside this run's draft (id 1), deleted"
check "a draft made beside another run's: the other's stays" "$(releases_now)" "v0.1.7:draft"
check "after the race, resumed on the other's" "$(github race-resumed 0.1.7)" 0
check "after the race: published" "$(github race-published 0.1.7 publish)" 0
check "after the race: one release" "$(releases_now)" "v0.1.7:published"
# Another clone's run between a read and a write: a draft published before abandon's deletion; an asset being
# replaced, or the draft retargeted, between the publication's checks and the publication.
fresh "$initial"
check "a draft for the race with abandon" "$(github race-abandon-draft 0.1.7)" 0
check "abandon of a draft published meanwhile" "$(STUB_PUBLISH_ON_READ=1 github abandon-raced 0.1.7 abandon)" 1
has "abandon of a draft published meanwhile: why" "$work/out.abandon-raced" "the release v0.1.7 (id 1) is published now: a published release is never deleted"
check "abandon of a draft published meanwhile: kept" "$(releases_now)" "v0.1.7:published"
fresh "$initial"
check "a draft for the races with publish" "$(github race-publish-draft 0.1.7)" 0
check "publish while an asset is replaced" "$(STUB_AFTER_DOWNLOAD='drop teq-0.1.7-linux-x86_64' github publish-dropped 0.1.7 publish)" 1
has "publish while an asset is replaced: why" "$work/out.publish-dropped" "the draft v0.1.7 is not whole now (teq-0.1.7-linux-x86_64 is not the file): another writer is at it"
check "publish while an asset is replaced: a draft, no tag" "$(releases_now) $(github_ref refs/tags/v0.1.7)" "v0.1.7:draft "
check "the draft resumed after the replacement" "$(github race-publish-resumed 0.1.7)" 0
check "publish while the draft is retargeted" "$(STUB_AFTER_DOWNLOAD="retarget $initial" github publish-retargeted 0.1.7 publish)" 1
has "publish while the draft is retargeted: why" "$work/out.publish-retargeted" "the draft v0.1.7 targets ${initial:0:12} now, not the release's commit ${release:0:12}"
check "publish while the draft is retargeted: a draft, no tag" "$(releases_now) $(github_ref refs/tags/v0.1.7)" "v0.1.7:draft "
check "the draft retargeted back" "$(github race-publish-back 0.1.7)" 0
check "publish after the races" "$(github race-publish 0.1.7 publish)" 0
check "the publication names the release's commit as its target" "$(grep -c -e "^release edit v0.1.7 .*--draft=false --target $release " "$work/stub/calls")" 1
check "publish after the races: the tag on the release's commit" "$(github_ref 'refs/tags/v0.1.7^{commit}')" "$release"
# Runs from one clone take turns.
fresh "$initial"
exec 8> "$repo/.git/github-release.lock"
flock 8
check "another run of the clone" "$(github locked 0.1.7 check)" 1
has "another run of the clone: why" "$work/out.locked" "another bench/github-release.sh runs from this clone"
exec 8>&-
# A tag of the checkout that push.followTags would take along stays behind.
fresh "$initial"
(cd "$repo" && git config push.followTags true && git tag -a -m old v0.1.7 "$initial")
check "push.followTags and a local tag: the draft" "$(github follow 0.1.7)" 0
check "push.followTags and a local tag: no tag pushed" "$(github_ref refs/tags/v0.1.7)" ""
check "push.followTags and a local tag: published" "$(github follow-publish 0.1.7 publish)" 0
check "push.followTags and a local tag: the tag on the release's commit" "$(github_ref 'refs/tags/v0.1.7^{commit}')" "$release"
(cd "$repo" && git config --unset push.followTags && git tag -d v0.1.7 > /dev/null)

# Refusals, each with exit 1 and nothing written.
# refused <label> <expected line> [args]: github-release.sh fails with the line, GitHub as it was.
refused() {
  local label=$1 line=$2 before
  shift 2
  before="$(releases_now) $(github_ref refs/heads/master) $(github_ref refs/tags/v0.1.7)"
  check "refused: $label" "$(github "$label" "$@")" 1
  has "refused: $label" "$work/out.$label" "$line"
  check "refused, GitHub as it was: $label" "$(releases_now) $(github_ref refs/heads/master) $(github_ref refs/tags/v0.1.7)" "$before"
}
fresh "$initial"
refused another-version "the checkout's release is 0.1.7, not 0.1.8" 0.1.8
printf '%s\n' '{"next": 3, "releases": [{"id": 1, "tag_name": "v0.1.7", "draft": true, "prerelease": false, "target_commitish": "x", "name": "a", "body": "", "assets": []}, {"id": 2, "tag_name": "v0.1.7", "draft": true, "prerelease": false, "target_commitish": "x", "name": "b", "body": "", "assets": []}]}' > "$work/stub/state.json"
refused two-releases "GitHub has 2 releases of v0.1.7" 0.1.7
refused two-releases-check "GitHub has 2 releases of v0.1.7" 0.1.7 check
refused two-releases-publish "GitHub has 2 releases of v0.1.7" 0.1.7 publish
printf '%s\n' '{"next": 2, "releases": [{"id": 1, "tag_name": "v0.1.7", "draft": true, "prerelease": false, "target_commitish": "x", "name": "a", "body": "", "assets": [{"name": "teq-extra.gz", "size": 1, "state": "uploaded", "digest": null}]}]}' > "$work/stub/state.json"
refused stray-asset "the release v0.1.7 holds teq-extra.gz, which it should not" 0.1.7
refused stray-asset-check "the release v0.1.7 holds teq-extra.gz, which it should not" 0.1.7 check
fresh "$initial"
git --git-dir="$work/github.git" tag v0.1.7 "$initial"
refused tag-elsewhere "GitHub's tag v0.1.7 is on ${initial:0:12}, not the release's commit" 0.1.7
fresh "$initial"
other=$(cd "$work" && rm -rf other && git init -q other && cd other && git commit -q --allow-empty -m "Add README and LICENSE" && git push -q "$work/github.git" HEAD:refs/heads/unrelated && git rev-parse HEAD)
git --git-dir="$work/github.git" update-ref refs/heads/master "$other"
refused diverged "GitHub's master ${other:0:12} is not on the hub's master ${release:0:12}: GitHub has commits the hub lacks" 0.1.7
refused diverged-check "GitHub's master ${other:0:12} is not on the hub's master" 0.1.7 check
check "diverged: the stand-in asked nothing of releases" "$(calls 'release ')" 0
fresh "$initial"
rm -rf "$binaries/linux-aarch_64"
refused held-back "nothing staged for linux-aarch_64 (held back, or not built): the release is the five binaries or none" 0.1.7
stage
printf 'x' >> "$binaries/osx-x86_64/teq"
refused not-its-manifest "integrations/sbt/binary/binaries/osx-x86_64/teq is not the binary its manifest records" 0.1.7
stage
sed -i "s/^commit .*/commit $initial/" "$binaries/linux-x86_64/teq.manifest"
refused other-commit "is built from ${initial:0:12}, not the release's commit" 0.1.7
stage
sed -i "s/^run .*/run ship-test-2/" "$binaries/windows-x86_64/teq.manifest"
refused two-stagings "the binaries were staged by different invocations" 0.1.7
stage
sed -i "s/^tuple rustc .*/tuple rustc 0.0.0/" "$binaries/osx-aarch_64/teq.manifest"
refused not-qualified "is not published by bench/ship-qualified.txt as it stands" 0.1.7
stage
rm -f "$repo/out/ship/profiles/teq-0.1.7-profiles.tar"
refused no-profiles "no out/ship/profiles/teq-0.1.7-profiles.tar, the profiles the binaries were guided by" 0.1.7
stage
tar -xf "$work/profiles/teq-0.1.7-profiles.tar" -C "$work" profiles.txt && tar -cf "$repo/out/ship/profiles/teq-0.1.7-profiles.tar" -C "$work" profiles.txt
refused profiles-not-whole "out/ship/profiles/teq-0.1.7-profiles.tar is refused" 0.1.7
stage
sed -i "s/^training profile .*/training profile $(printf '%064d' 7)/" "$binaries/osx-x86_64/teq.manifest"
refused profiles-not-guiding "does not hold the x86_64 profile that guided osx-x86_64" 0.1.7
stage
# A commit without NOTICE: refused before anything is written (the dry run's, whose commit is the head).
(cd "$repo" && git rm -q NOTICE && git commit -qm "Drop NOTICE")
refused no-notice "has no NOTICE, which the release carries beside its binaries" --dry-run
(cd "$repo" && git reset -q --hard HEAD~1)
chmod 644 "$work/home/.config/teq/github-token"
refused token-mode "github-token has mode 644, not 600" 0.1.7
chmod 600 "$work/home/.config/teq/github-token"
mv "$work/home/.config/teq/github-token" "$work/token.aside"
refused no-token "no $work/home/.config/teq/github-token" 0.1.7
mv "$work/token.aside" "$work/home/.config/teq/github-token"
# A head after the release's commit, and a release's commit the hub does not hold.
(cd "$repo" && git commit -q --allow-empty -m Later && git push -q origin master)
refused not-the-bump "is not the commit that bumped the version to 0.1.7" 0.1.7
(cd "$repo" && git reset -q --hard "$release")
(cd "$repo" && sed -i 's/0\.1\.7/0.1.8/' Cargo.toml && sed -i '/^name = "teq"$/{n;s/0\.1\.7/0.1.8/}' Cargo.lock && git commit -qam "Release 0.1.8")
refused not-on-the-hub "is not on the hub's master" 0.1.8
(cd "$repo" && git reset -q --hard "$release")
(cd "$repo" && sed -i 's/A change for users,/A change the Codex pass found,/' CHANGELOG.md && git commit -qam "Note badly" && git reset -q --soft HEAD~2 && git commit -qm "Release 0.1.7 with a bad note" && git push -qf origin master)
refused process-words "names the work's process or its internal files: Codex" 0.1.7
(cd "$repo" && git reset -q --hard "$release" && git push -qf origin master)

# The hub's master moving after this run read it: refused before the push, GitHub's master left.
fresh "$initial"
(cd "$work" && rm -rf moved && git clone -q "$work/hub.git" moved && cd moved && git commit -q --allow-empty -m Landed && git push -q origin HEAD:refs/heads/landed)
landed=$(git --git-dir="$work/hub.git" rev-parse refs/heads/landed)
check "the hub's master moving before the push" "$(STUB_MOVE_HUB=$landed STUB_HUB=$work/hub.git github hub-moved 0.1.7)" 1
has "the hub's master moving before the push: why" "$work/out.hub-moved" "the hub's master is ${landed:0:12} now, not the ${release:0:12} this run read"
check "the hub's master moving before the push: GitHub's master left, no release" "$(github_ref refs/heads/master) $(releases_now)" "$initial "
git --git-dir="$work/hub.git" update-ref refs/heads/master "$release"

# The hub's master past the release's commit: pushed whole, the tag still on the release's commit.
fresh "$initial"
(cd "$work" && rm -rf later && git clone -q "$work/hub.git" later && cd later && git commit -q --allow-empty -m Later && git push -q origin master)
later=$(git --git-dir="$work/hub.git" rev-parse master)
check "the hub's master past the release" "$(github past 0.1.7) $(github past-publish 0.1.7 publish)" "0 0"
check "the hub's master past the release: pushed" "$(github_ref refs/heads/master)" "$later"
check "the hub's master past the release: the tag" "$(github_ref 'refs/tags/v0.1.7^{commit}')" "$release"
git --git-dir="$work/hub.git" update-ref refs/heads/master "$release"

# --dry-run: the scratch draft made, stopped, resumed, abandoned; an earlier one's left draft deleted first.
fresh "$initial"
check "the dry run" "$(github dry --dry-run)" 0
has "the dry run: stopped" "$work/out.dry" "stopped after 1 upload(s)"
has "the dry run: resumed" "$work/out.dry" "the second pass resumed the draft 1: one release of v0.0.0-dry"
has "the dry run: the assets checked" "$work/out.dry" "the assets of the draft release v0.0.0-dry downloaded back: each the file, SHA256SUMS and teq-0.0.0-dry-binaries.txt theirs"
has "the dry run: abandoned" "$work/out.dry" "abandoned the draft v0.0.0-dry: no release of it left, no tag"
check "the dry run: nothing left" "$(releases_now)" ""
check "the dry run: made once, uploaded ten" "$(calls 'api -X POST') $(calls 'release upload')" "1 10"
check "the dry run: no tag" "$(github_ref refs/tags/v0.0.0-dry)" ""
check "the dry run: GitHub's master left" "$(github_ref refs/heads/master)" "$initial"
printf '%s\n' '{"next": 9, "releases": [{"id": 8, "tag_name": "v0.0.0-dry", "draft": true, "prerelease": true, "target_commitish": "x", "name": "a", "body": "", "assets": []}]}' > "$work/stub/state.json"
check "the dry run after a left draft" "$(github dry-left --dry-run)" 0
has "the dry run after a left draft: deleted first" "$work/out.dry-left" "deleted the draft v0.0.0-dry (id 8)"
check "the dry run after a left draft: nothing left" "$(releases_now)" ""
git --git-dir="$work/github.git" update-ref refs/heads/master "$other"
check "the dry run with GitHub's master diverged: 3, its draft written" "$(github dry-diverged --dry-run)" 3
has "the dry run with GitHub's master diverged: the draft gone" "$work/out.dry-diverged" "abandoned the draft v0.0.0-dry"
has "the dry run with GitHub's master diverged: why" "$work/out.dry-diverged" "the dry run passed but for the push"
has "the dry run with GitHub's master diverged: what it wrote" "$work/out.dry-diverged" "this dry run wrote its scratch draft to GitHub, deleted on its way out"
check "the dry run with GitHub's master diverged: nothing left" "$(releases_now)" ""

# A GitHub Actions run: GitHub's master not pushed (the run is on it), the token GH_TOKEN's without the file, the
# file preferred when present; outside such a run GH_TOKEN alone is no token.
mkdir -p "$work/home-actions"
fresh "$release"
check "GH_TOKEN alone outside a GitHub Actions run" "$(GHOME=$work/home-actions ENVTOKEN=$token github env-token 0.1.7 check)" 1
has "GH_TOKEN alone outside a GitHub Actions run: why" "$work/out.env-token" "no $work/home-actions/.config/teq/github-token, the maintainer's GitHub token (mode 600)"
check "a GitHub Actions run without a token" "$(GHOME=$work/home-actions ACTIONS=1 github actions-none 0.1.7 check)" 1
has "a GitHub Actions run without a token: why" "$work/out.actions-none" "and no GH_TOKEN in this GitHub Actions run"
check "a GitHub Actions run: the file preferred to GH_TOKEN" "$(ACTIONS=1 ENVTOKEN=ghp_another github actions-file 0.1.7 check)" 0
check "a GitHub Actions run: the draft" "$(GHOME=$work/home-actions ACTIONS=1 ENVTOKEN=$token github actions-draft 0.1.7)" 0
has "a GitHub Actions run: master not pushed" "$work/out.actions-draft" "a GitHub Actions run: GitHub's master ${release:0:12} is not pushed"
check "a GitHub Actions run: published" "$(GHOME=$work/home-actions ACTIONS=1 ENVTOKEN=$token github actions-publish 0.1.7 publish)" 0
check "a GitHub Actions run: the tag on the release's commit" "$(github_ref 'refs/tags/v0.1.7^{commit}')" "$release"
fresh "$initial"
check "a GitHub Actions run with GitHub's master behind" "$(GHOME=$work/home-actions ACTIONS=1 ENVTOKEN=$token github actions-behind 0.1.7)" 1
has "a GitHub Actions run with GitHub's master behind: why" "$work/out.actions-behind" "GitHub's master ${initial:0:12} does not hold the release's commit ${release:0:12}, and a GitHub Actions run pushes nothing"
check "a GitHub Actions run with GitHub's master behind: no release" "$(releases_now)" ""
check "a GitHub Actions run with GitHub's master behind: nothing pushed" "$(github_ref refs/heads/master)" "$initial"

# Git's HTTP tracing, inherited from the caller, over a server that asks for the credential (401, then 404 to
# the request that brings it): a plain git with the script's helper traces the credential, the script does not.
cat > "$work/challenge.py" << 'PY'
import http.server, sys
class Challenge(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.headers.get("Authorization"):
            open(sys.argv[2], "a").write("authorization\n")
            self.send_response(404)
        else:
            self.send_response(401)
            self.send_header("WWW-Authenticate", 'Basic realm="teq"')
        self.send_header("Content-Length", "0")
        self.end_headers()
    def log_message(self, *args):
        pass
http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), Challenge).serve_forever()
PY
hport=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
python3 "$work/challenge.py" "$hport" "$work/challenged" > /dev/null 2>&1 &
challenger=$!
for _ in $(seq 50); do curl -s -o /dev/null --max-time 1 "http://127.0.0.1:$hport/" && break; sleep 0.1; done
grep -v -e '^\[url ' -e 'insteadOf' "$GIT_CONFIG_GLOBAL" > "$work/gitconfig-http"
printf '[url "http://127.0.0.1:%s/teq.git"]\n\tinsteadOf = https://github.com/Carrot-Inc/teq.git\n' "$hport" >> "$work/gitconfig-http"
helper=$(sed -n "s/^.*-c '\(credential\.helper=!f() .*; f\)' .*\$/\1/p" bench/github-release.sh)
basic=$(printf 'x-access-token:%s' "$token" | base64 -w0)
(cd "$repo" && GIT_CONFIG_GLOBAL=$work/gitconfig-http GIT_TRACE_CURL=1 GIT_TRACE_REDACT=0 TEQ_GITHUB_TOKEN_FILE="$work/home/.config/teq/github-token" \
  GIT_TERMINAL_PROMPT=0 timeout 30 git -c credential.helper= -c "$helper" ls-remote https://github.com/Carrot-Inc/teq.git > "$work/traced" 2>&1)
check "git's HTTP tracing records the credential the helper hands (the control)" "$(grep -q -F "$basic" "$work/traced" && echo traced)" traced
fresh "$release"
: > "$work/challenged"
check "the script under git's HTTP tracing" \
  "$(GIT_CONFIG_GLOBAL=$work/gitconfig-http GIT_TRACE_CURL=1 GIT_TRACE_REDACT=0 GIT_TRACE=1 GIT_CURL_VERBOSE=1 GIT_TRACE_PACKET=1 github git-traced 0.1.7 check)" 1
check "the script under git's HTTP tracing: the credential asked for and sent" "$(sort -u "$work/challenged")" authorization
check "the script under git's HTTP tracing: the credential in no output" "$(grep -c -F -e "$basic" -e "$token" "$work/out.git-traced")" 0
kill "$challenger" 2> /dev/null

# An https origin that is GitHub, as in a GitHub Actions checkout, under the caller's tracing: git's smart HTTP
# (git http-backend) behind a server that answers 401 to a request without a credential. Origin's credential
# comes from its own configuration: a helper after the challenge, or a header persisted as actions/checkout
# leaves it (http.extraheader). The hub one commit past the checkout, so that the run fetches over HTTP too.
cat > "$work/origin.py" << 'PY'
import http.server, os, subprocess, sys
port, root, log = int(sys.argv[1]), sys.argv[2], sys.argv[3]
backend = os.path.join(subprocess.run(["git", "--exec-path"], capture_output=True, text=True).stdout.strip(), "git-http-backend")
class Origin(http.server.BaseHTTPRequestHandler):
    def serve(self):
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        if not self.headers.get("Authorization"):
            self.send_response(401)
            self.send_header("WWW-Authenticate", 'Basic realm="teq"')
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        path, _, query = self.path.partition("?")
        open(log, "a").write("authorization %s %s\n" % (self.command, path.rsplit("/", 1)[-1]))
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_TRACE")}
        env.update(GIT_PROJECT_ROOT=root, GIT_HTTP_EXPORT_ALL="1", PATH_INFO=path, QUERY_STRING=query, REQUEST_METHOD=self.command,
                   CONTENT_TYPE=self.headers.get("Content-Type", ""), CONTENT_LENGTH=str(len(body)), REMOTE_ADDR="127.0.0.1",
                   HTTP_GIT_PROTOCOL=self.headers.get("Git-Protocol", ""), HTTP_CONTENT_ENCODING=self.headers.get("Content-Encoding", ""))
        out = subprocess.run([backend], input=body, env=env, capture_output=True, timeout=60).stdout
        sep = b"\r\n\r\n" if b"\r\n\r\n" in out else b"\n\n"
        head, _, rest = out.partition(sep)
        status, headers = 200, []
        for line in head.decode().splitlines():
            k, _, v = line.partition(": ")
            if k.lower() == "status":
                status = int(v.split()[0])
            elif k:
                headers.append((k, v))
        self.send_response(status)
        for k, v in headers:
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(rest)))
        self.end_headers()
        self.wfile.write(rest)
    do_GET = do_POST = serve
    def log_message(self, *args):
        pass
http.server.ThreadingHTTPServer(("127.0.0.1", port), Origin).serve_forever()
PY
oport=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
env -u GIT_CONFIG_GLOBAL python3 "$work/origin.py" "$oport" "$work" "$work/origin-log" > /dev/null 2>&1 &
originserver=$!
for _ in $(seq 50); do curl -s -o /dev/null --max-time 1 "http://127.0.0.1:$oport/" && break; sleep 0.1; done
(cd "$work" && rm -rf ahead && git clone -q "$work/hub.git" ahead)
(cd "$repo" && git remote set-url origin https://github.com/Carrot-Inc/teq.git)
# origin_config <file> <helper | header>: the run's global configuration, GitHub's URL served by the server.
origin_config() {
  printf '[user]\n\tname = test\n\temail = test@example.invalid\n[url "http://127.0.0.1:%s/hub.git"]\n\tinsteadOf = https://github.com/Carrot-Inc/teq.git\n' "$oport" > "$1"
  case $2 in
    helper) git config --file "$1" credential.helper "!f() { test \"\$1\" = get || return 0; echo username=x-access-token; echo password=$token; }; f" ;;
    header) git config --file "$1" http.extraheader "AUTHORIZATION: basic $basic" ;;
  esac
}
for variant in helper header; do
  (cd "$work/ahead" && git commit -q --allow-empty -m "Ahead for the $variant" && git push -q origin HEAD:master)
  origin_config "$work/gitconfig-origin-$variant" "$variant"
  : > "$work/origin-log"
  fresh "$release"
  check "an https origin in a GitHub Actions run, its credential by a $variant, under the caller's tracing" \
    "$(GIT_CONFIG_GLOBAL=$work/gitconfig-origin-$variant GIT_TRACE_CURL=1 GIT_TRACE_REDACT=0 GIT_TRACE=1 GIT_CURL_VERBOSE=1 GIT_TRACE_PACKET=1 GH_DEBUG=api \
      GHOME=$work/home-actions ACTIONS=1 ENVTOKEN=$token github "origin-$variant" 0.1.7 check)" 0
  check "an https origin by a $variant: origin read and fetched over HTTP with the credential" \
    "$(grep -c '^authorization POST git-upload-pack$' "$work/origin-log" | sed 's/^[1-9][0-9]*$/fetched/')" fetched
  check "an https origin by a $variant: the credential in no output" "$(grep -c -F -e "$basic" -e "$token" "$work/out.origin-$variant")" 0
done
kill "$originserver" 2> /dev/null
(cd "$repo" && git remote set-url origin "$work/hub.git")
git --git-dir="$work/hub.git" update-ref refs/heads/master "$release"

# The token: in no output, trace or argument of gh, never stored by git; the script's helper hands it.
fresh "$initial"
check "under bash -x" "$(XTRACE=1 github xtrace 0.1.7 check)" 0
check "the token in no output or trace" "$(grep -c -F "$token" "$work/outputs")" 0
check "the token in no argument of gh" "$(cat "$work"/stub/calls 2> /dev/null | grep -c -F "$token")" 0
check "the token stored by no credential helper" "$([ -e "$work/stored-credentials" ] && echo stored)" ""
helper=$(sed -n "s/^.*-c '\(credential\.helper=!f() .*; f\)' .*\$/\1/p" bench/github-release.sh)
filled=$(printf 'protocol=https\nhost=github.com\n\n' | TEQ_GITHUB_TOKEN_FILE="$work/home/.config/teq/github-token" GIT_TERMINAL_PROMPT=0 \
  timeout 30 git -c credential.helper= -c "$helper" credential fill 2> /dev/null | grep -c -x -F -e "password=$token" -e username=x-access-token)
check "the credential helper hands the token" "$filled" 2
filled=$(printf 'protocol=https\nhost=github.com\n\n' | TEQ_GITHUB_TOKEN_FILE= GH_TOKEN="$token" GIT_TERMINAL_PROMPT=0 \
  timeout 30 git -c credential.helper= -c "$helper" credential fill 2> /dev/null | grep -c -x -F -e "password=$token" -e username=x-access-token)
check "the credential helper hands GH_TOKEN without the file" "$filled" 2
printf 'protocol=https\nhost=github.com\nusername=x-access-token\npassword=%s\n\n' "$token" | TEQ_GITHUB_TOKEN_FILE="$work/home/.config/teq/github-token" \
  timeout 30 git -c credential.helper= -c "$helper" credential approve
check "the credential helper stores nothing, the configured one set aside" "$([ -e "$work/stored-credentials" ] && echo stored)" ""

echo "release scripts: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
