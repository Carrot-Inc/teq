#!/bin/bash
# Checks of the release workflow's scripts without GitHub, the Portal or a build (docs/DEVELOPING.md, "Releases"):
# the workflows' shape (actions pinned by digest, no pull_request_target, permissions none by default and written
# per job, the secrets in the `release` environment's steps alone, every job on the admitted commit), actionlint
# where installed; bench/ship.sh's steps run alone from the products they carry (stand-ins for the builds);
# tests/support/identity.sh's produce and compare modes (a stand-in compiler) and their refusals; the admission
# (bench/actions/admit.sh) against local repositories, a stand-in gh and a local Central; the publication's steps,
# bench/ship-publish.sh --step through bench/actions/release-step.sh with stand-ins of the GitHub release, the
# smoke, the plugin's publish and the Portal, its records carried to another runner where a job is lost, its
# conclusion and the pin's push onto a moving master; the landing's guard (bench/actions/before-landing.sh); the
# restore of the built jars; the credentials, with a key made for the run. Each check prints FAIL with what it saw;
# the script exits 1 on a failure. Every command is bounded; KEEP=1 keeps the scratch directory.
cd "$(dirname "$0")/.." || exit 1
root=$PWD
work=$(mktemp -d "${TMPDIR:-/tmp}/release-actions.XXXXXX") || exit 1
pids=
trap 'for p in $pids; do kill "$p" 2> /dev/null; done; [ -n "${KEEP:-}" ] || rm -rf -- "$work"' EXIT
pass=0
fail=0
check() {
  if [ "$2" = "$3" ]; then pass=$((pass + 1)); else echo "FAIL $1: '$2', where '$3' is expected"; fail=$((fail + 1)); fi
}
has() {
  if grep -qF -- "$3" "$2"; then pass=$((pass + 1)); else echo "FAIL $1: no '$3' in:"; sed 's/^/  /' "$2" | tail -15; fail=$((fail + 1)); fi
}
lacks() {
  if ! grep -qF -- "$3" "$2"; then pass=$((pass + 1)); else echo "FAIL $1: '$3' in $2"; fail=$((fail + 1)); fi
}
git_() { git -c user.name=t -c user.email=t@example.invalid -c commit.gpgsign=false -c init.defaultBranch=master "$@"; }
export GIT_CONFIG_NOSYSTEM=1

# --- The workflows' shape.
for f in bench/actions/*.sh tests/windows.sh tests/support/identity.sh tests/support/identity-across.sh bench/ship.sh; do
  check "bash -n $f" "$(bash -n "$f" 2>&1)" ""
done
for f in .github/workflows/release.yml .github/workflows/ship-image.yml; do
  check "$f: every action pinned by a commit" "$(grep -E '^\s*(- )?uses:' "$f" | grep -vE 'uses: [a-z0-9-]+/[a-z0-9-]+@[0-9a-f]{40} # v[0-9.]+$')" ""
  check "$f: no pull_request trigger" "$(grep -cE 'pull_request' "$f")" 0
  check "$f: no permission by default" "$(grep -c '^permissions: {}$' "$f")" 1
done
if command -v actionlint > /dev/null; then
  check "actionlint" "$(timeout 120 actionlint .github/workflows/release.yml .github/workflows/ship-image.yml 2>&1)" ""
else
  echo "skip actionlint: not installed"
fi
if python3 -c 'import yaml' 2> /dev/null; then
  shape=$(python3 - << 'PY'
import re, yaml
w = yaml.safe_load(open(".github/workflows/release.yml"))
out = []
on = w.get(True) or w.get("on")
if set(on) != {"push", "workflow_dispatch"} or on["push"].get("branches") != ["master"]:
    out.append("triggers: %s" % list(on))
writers = set()
for name, job in w["jobs"].items():
    perms = job.get("permissions")
    if perms is None:
        out.append(f"{name}: no permissions of its own")
        continue
    if perms.get("contents") == "write":
        writers.add(name)
    if "secrets." in str(job.get("env", {})):
        out.append(f"{name}: a secret in the job's environment")
    for step in job.get("steps", []):
        text = str(step.get("env", {})) + str(step.get("with", {}))
        if "secrets." in text:
            if job.get("environment") != "release":
                out.append(f"{name}: a secret outside the release environment")
            run = re.sub(r"^timeout \d+ ", "", step.get("run", ""))
            if not (run.startswith("bench/actions/credentials.sh write") or run == "bench/actions/release-step.sh site"):
                out.append(f"{name}: a secret given to '{run or step.get('uses')}'")
        if str(step.get("uses", "")).startswith("actions/checkout@"):
            w_ = step.get("with", {})
            if w_.get("persist-credentials") is not False:
                out.append(f"{name}: a checkout that keeps its credentials")
            if name not in ("admit",) and w_.get("ref") not in ("${{ needs.admit.outputs.commit }}", "${{ needs.pin.outputs.commit }}"):
                out.append(f"{name}: a checkout of {w_.get('ref')}, not the admitted commit")
    if name != "admit" and "admit" not in (job.get("needs") if isinstance(job.get("needs"), list) else [job.get("needs")]):
        out.append(f"{name}: does not follow the admission")
if writers != {"draft", "central-promote", "publish", "pin", "conclude"}:
    out.append("contents: write in %s" % sorted(writers))
print("\n".join(out))
PY
)
  check "release.yml: permissions, secrets and checkouts" "$shape" ""
  # Every job's limit holds the bounds of its commands (each `timeout N`; bench/ship.sh's steps by their own bounds,
  # the corpus counted as the ship counts it, both trainings native as the workflow's runners make them, with the
  # warm-up of the built jars where a suite runs) and twenty minutes for a cold setup and the transfers (the image
  # pulled, the checkout, the artifacts; the caches' misses are inside the bounds of their fetches).
  bounds=$(timeout 120 python3 - << 'PY'
import re, subprocess, yaml
w = yaml.safe_load(open(".github/workflows/release.yml"))
ship = open("bench/ship.sh").read()
variants = subprocess.check_output(["python3", "bench/features.py", "--names"], text=True).split()
programs = len(variants) + 2 + len(re.findall(r"^with_jars ", open("bench/programs.sh").read(), re.M))
run = int(re.search(r"^  train_bound=\$\(\( 600 \+ \(programs \+ 8\) \* (\d+) \)\)$", ship, re.M).group(1))
train = 600 + (programs + 8) * run
bounds = {}
for name, expr in re.findall(r'^wanted (\S+) && step \S+ (\$\(\(.*?\)\)|"\$train_bound") ', ship, re.M):
    bounds[name] = train if expr == '"$train_bound"' else eval(expr[3:-2].replace("arm_train_bound", str(train)).replace("train_bound", str(train)))
bounds["stage"] = sum(int(x) for x in re.findall(r"^step stage-\S+ (\d+) ", ship, re.M)) + \
    int(re.search(r'timeout (\d+) "\$root/integrations/sbt/binary/check.sh"', ship).group(1))
warm = int(re.search(r"timeout (\d+) bash -c '. tests/support/jars.sh && jars_warm'", ship).group(1))
out = []
for name, job in w["jobs"].items():
    for leg in job.get("strategy", {}).get("matrix", {}).get("include", [None]):
        # The steps of a training and those of profiles taken up are alternatives: the larger counts.
        totals = {"": 0, "train": 0, "take": 0}
        for step in job.get("steps", []):
            run = step.get("run", "").replace("$STEP", (leg or {}).get("step", "$STEP"))
            cond = str(step.get("if", ""))
            way = "train" if "outputs.train == 'true'" in cond else "take" if "outputs.train != 'true'" in cond else ""
            totals[way] += sum(int(x) for x in re.findall(r"\b(?:timeout|alarm) (\d+)\b", run))
            m = re.search(r'bench/actions/step\.sh "?([a-z,-]+)"?', run)
            if m:
                steps = m.group(1).split(",")
                totals[way] += sum(bounds[x] for x in steps) + 600 + (warm if set(steps) & {"pgo-use", "cross-linux-arm"} else 0)
        total = totals[""] + max(totals["train"], totals["take"])
        minutes = job["timeout-minutes"]
        minutes = leg["minutes"] if isinstance(minutes, str) else minutes
        if minutes > 360 or minutes * 60 < total + 1200:
            out.append("%s%s: %d minutes for bounds of %d and the 20 of the setup" % (name, " " + str(leg) if leg else "", minutes, -(-total // 60)))
print("\n".join(out))
PY
)
  check "release.yml: every job's limit holds its commands' bounds" "$bounds" ""
  # Inside them, the GitHub release's verbs: bench/ship-release.sh's bounds hold github-release.sh's own over every
  # file release_assets names (an upload each in the draft, a read-back each in the publication), bench/ship-publish.sh
  # bounds the verbs by them, and the workflow's steps that run them hold them and the rest of their commands.
  verbs=$(timeout 120 python3 - << 'PY'
import re, subprocess, yaml
gr = open("bench/github-release.sh").read()
sp = open("bench/ship-publish.sh").read()
rs = open("bench/actions/release-step.sh").read()
sh = lambda c: subprocess.check_output(["bash", "-c", ". bench/ship-release.sh && " + c], text=True).strip()
files = len(sh("release_assets 0.1.7").split())
upload = int(re.search(r"gh_ (\d+) release upload", gr).group(1))
back = int(re.search(r"gh_ (\d+) release download", gr).group(1))
tag = int(re.search(r"git_ (\d+) ls-remote", gr).group(1))
polls = re.search(r"for try in ([\d ]+); do\n\s*tag_at=\$\(github_tag\)", gr).group(1).split()
pause = int(re.search(r"tag_at=\$\(github_tag\).*?sleep (\d+)", gr, re.S).group(1))
tries = re.search(r"for try in ([\d ]+); do\n\s*\[ \"\$try\" -eq 1 \] \|\| sleep (\d+)", gr)
maxtime = int(re.search(r"curl -sSfL --max-time (\d+) -o \"\$work/public", gr).group(1))
reads, wait = len(tries.group(1).split()), int(tries.group(2))
draft, publish = int(sh("release_draft_bound 0.1.7")), int(sh("release_publish_bound 0.1.7"))
out = []
if 'names=$(release_assets "$version"' not in gr or files != 10:
    out.append(f"github-release.sh's files are not release_assets' {files}")
if draft < 900 + files * upload + back:
    out.append(f"release_draft_bound {draft} < 900 + {files} uploads of {upload} + {back}")
if publish < 900 + back + len(polls) * (tag + pause) + files * (reads * maxtime + (reads - 1) * wait):
    out.append(f"release_publish_bound {publish} < the publication's own bounds over {files} files")
if "draft_bound=$(release_draft_bound" not in sp or "publish_bound=$(release_publish_bound" not in sp or \
        len(re.findall(r"gh_release \$draft_bound", sp)) != 2 or "gh_release $publish_bound publish" not in sp:
    out.append("bench/ship-publish.sh does not bound the verbs by release_draft_bound and release_publish_bound")
steps = {}
for job in yaml.safe_load(open(".github/workflows/release.yml"))["jobs"].values():
    for st in job.get("steps", []):
        m = re.search(r"timeout (\d+) bench/actions/release-step\.sh (draft|publish \S+)$", st.get("run", "").strip())
        if m:
            steps[m.group(2)] = int(m.group(1))
against = int(re.search(r'timeout (\d+) gh release download "\$tag"', rs).group(1))
portal = sum(int(x) for x in re.findall(r"timeout (\d+) python3 -B integrations/sbt/central\.py (?:wait|promote)", sp))
for step, need in (("draft", 1000 + draft + against), ("publish promote", draft + portal), ("publish publish", publish)):
    if steps.get(step, 0) < need:
        out.append(f"release-step.sh {step}: {steps.get(step)} s for bounds of {need}")
print("\n".join(out))
PY
)
  check "the GitHub release's verbs bounded over every file of the release, and their steps by them" "$verbs" ""
  # The Central jobs persist the record and the Central record, in steps that always run, before each step that
  # writes to the Portal: the staging before the upload, the intent before it, the deployment before the promotion.
  checkpoints=$(timeout 60 python3 - << 'PY'
import yaml
w = yaml.safe_load(open(".github/workflows/release.yml"))
def persisted(steps, lo, hi, names):
    found = {s.get("with", {}).get("name") for s in steps[lo + 1:hi]
             if str(s.get("uses", "")).startswith("actions/upload-artifact@") and s.get("if") == "always()"}
    return set(names) <= found
out = []
c = w["jobs"]["central"]["steps"]
runs = [s.get("run", "") for s in c]
at = lambda text: next(i for i, r in enumerate(runs) if r.rstrip().endswith(text))
stage, begin, upload = at("publish stage,smoke"), at("publish upload-begin"), at("publish upload")
if not stage < begin < upload:
    out.append("central: the phases out of order")
if not persisted(c, stage, begin, ["central-record", "ship-record"]):
    out.append("central: the staging not persisted before the upload's intent")
if not persisted(c, begin, upload, ["ship-record"]):
    out.append("central: the intent not persisted before the upload")
if not persisted(c, upload, len(c), ["central-record", "ship-record"]):
    out.append("central: the deployment not persisted after the upload")
# The promotion and the read-back, each in a job of its own after the one before: the records taken up before
# their step, persisted after it.
for name, before, step in (("central-promote", "central", "publish promote"), ("central-readback", "central-promote", "publish read-back")):
    job = w["jobs"][name]
    p = job["steps"]
    runs = [s.get("run", "") for s in p]
    i = next(i for i, r in enumerate(runs) if r.rstrip().endswith(step))
    taken = {s.get("with", {}).get("name") for s in p[:i] if "download-artifact" in str(s.get("uses", ""))}
    if before not in job["needs"] or not {"central-record", "ship-record"} <= taken:
        out.append(f"{name}: the records not taken up before '{step}'")
    if not persisted(p, i, len(p), ["central-record", "ship-record"]):
        out.append(f"{name}: '{step}' not persisted")
print("\n".join(out))
PY
)
  check "release.yml: the Central records persisted before each write to the Portal" "$checkpoints" ""
  # Every job's runner is a repository variable's, GitHub's standard runner the default (docs/DEVELOPING.md,
  # "Releases"); a job in an image runs on a runner of the image's architecture; the caches are where the scripts
  # read them (zig's TEQ_ZIG_CACHE, coursier's COURSIER_CACHE), zig's keyed by the runner's architecture.
  runners=$(timeout 60 python3 - << 'PY'
import re, yaml
defaults = {"TEQ_RUNNER_LINUX_X64": "ubuntu-24.04", "TEQ_RUNNER_LINUX_ARM64": "ubuntu-24.04-arm",
            "TEQ_RUNNER_MACOS_ARM64": "macos-15", "TEQ_RUNNER_MACOS_X64": "macos-15-intel", "TEQ_RUNNER_WINDOWS": "windows-2025"}
form = re.compile(r"^\$\{\{ vars\.(TEQ_RUNNER_[A-Z0-9_]+) \|\| '([a-z0-9.-]+)' \}\}$")
out = []
def variable(where, label):
    m = form.match(str(label))
    if not m or defaults.get(m.group(1)) != m.group(2):
        out.append(f"{where}: runs on {label!r}, not a variable with its default")
        return None
    return m.group(1)
for f in (".github/workflows/release.yml", ".github/workflows/ship-image.yml"):
    w = yaml.safe_load(open(f))
    for name, job in w["jobs"].items():
        where = f"{f}: {name}"
        if job["runs-on"] == "${{ matrix.runner }}":
            got = [variable(f"{where} {leg}", leg["runner"]) for leg in job["strategy"]["matrix"]["include"]]
            if sorted(g or "" for g in got) != sorted(defaults):
                out.append(f"{where}: the qualification's runners are {got}")
            continue
        var = variable(where, job["runs-on"])
        image = str(job.get("container", {}).get("image", ""))
        if image:
            want = "TEQ_RUNNER_LINUX_ARM64" if "image_arm64" in image else "TEQ_RUNNER_LINUX_X64"
            if var != want:
                out.append(f"{where}: its image {image} on {var}'s runner")
            env = job.get("env", {})
            if env.get("TEQ_SHIP_IMAGE") not in (None, image):
                out.append(f"{where}: TEQ_SHIP_IMAGE {env.get('TEQ_SHIP_IMAGE')} in the image {image}")
        for step in job.get("steps", []):
            if not str(step.get("uses", "")).startswith("actions/cache@"):
                continue
            path, key = step["with"]["path"], step["with"]["key"]
            env = job.get("env", {})
            if path.endswith("/.cache/zig"):
                if env.get("TEQ_ZIG_CACHE") != path or "${{ runner.arch }}" not in key:
                    out.append(f"{where}: zig's cache {path} ({key}), TEQ_ZIG_CACHE {env.get('TEQ_ZIG_CACHE')}")
            elif path.endswith("/.cache/coursier/v1"):
                if image and (path != "/github/home/.cache/coursier/v1" or env.get("COURSIER_CACHE") != path):
                    out.append(f"{where}: coursier's cache {path} in the image, COURSIER_CACHE {env.get('COURSIER_CACHE')}")
                if not image and path != "~/.cache/coursier/v1":
                    out.append(f"{where}: coursier's cache {path} on the runner")
print("\n".join(out))
PY
)
  check "release.yml: the runners by variables, each image on its architecture, the caches where the scripts read them" "$runners" ""
  # Each qualification as soon as its binary exists: five builds, each uploading its own products; each qualification
  # needing its build (and the reference where it compares outputs with it) and downloading that alone, on its
  # platform's runner; the stage beside them, needing the builds and the Linux aarch64 suite; the draft needing the
  # stage and the five; the conclusion every job.
  graph=$(timeout 60 python3 - << 'PY'
import yaml
w = yaml.safe_load(open(".github/workflows/release.yml"))
jobs = w["jobs"]
out = []
def needs(name):
    n = jobs[name].get("needs", [])
    return set(n if isinstance(n, list) else [n])
def downloads(name):
    return {(st.get("with", {}).get("name") or st.get("with", {}).get("pattern")) for st in jobs[name]["steps"] if "download-artifact" in str(st.get("uses", ""))}
builds = {"linux-x64": "linux-x86_64", "linux-arm64": "linux-aarch_64", "macos-arm64": "osx-aarch_64", "macos-x64": "osx-x86_64", "windows": "windows-x86_64"}
runners = {"linux-x64": "TEQ_RUNNER_LINUX_X64", "linux-arm64": "TEQ_RUNNER_LINUX_ARM64", "macos-arm64": "TEQ_RUNNER_MACOS_ARM64", "macos-x64": "TEQ_RUNNER_MACOS_X64", "windows": "TEQ_RUNNER_WINDOWS"}
for b, classifier in builds.items():
    build, qualify = "build-" + b, "qualify-" + b
    if build not in jobs or qualify not in jobs:
        out.append(f"no {build} or {qualify}")
        continue
    if not any(st.get("with", {}).get("name") == "carry-" + build for st in jobs[build]["steps"] if "upload-artifact" in str(st.get("uses", ""))):
        out.append(f"{build}: uploads no carry-{build}")
    identity = classifier.startswith("osx") or classifier == "linux-aarch_64"
    want = {"admit", build} | ({"reference"} if identity else set())
    if needs(qualify) != want:
        out.append(f"{qualify}: needs {sorted(needs(qualify))}, not {sorted(want)}")
    if downloads(qualify) != {"carry-" + build} | ({"identity-reference"} if identity else set()):
        out.append(f"{qualify}: downloads {sorted(downloads(qualify))}")
    if runners[b] not in str(jobs[qualify]["runs-on"]) or classifier not in str(jobs[qualify]["steps"]):
        out.append(f"{qualify}: not {classifier} on {runners[b]}'s runner")
stage = needs("stage")
if not {"build-" + b for b in builds} | {"suite-linux-arm64"} <= stage or any(n.startswith("qualify") for n in stage):
    out.append(f"stage: needs {sorted(stage)}")
if needs("draft") != {"admit", "profiles", "stage"} | {"qualify-" + b for b in builds}:
    out.append(f"draft: needs {sorted(needs('draft'))}")
missing = set(jobs) - {"admit", "conclude"} - needs("conclude")
if missing:
    out.append(f"conclude: does not need {sorted(missing)}")
print("\n".join(out))
PY
)
  check "release.yml: each qualification as soon as its build, the stage beside them, the draft after all" "$graph" ""
  # The images by Depot's builders when TEQ_DEPOT_PROJECT is set, its ID in the variable alone, trusted by the run's
  # OIDC token; else by buildx here; never both.
  depot=$(timeout 60 python3 - << 'PY'
import yaml
jobs = yaml.safe_load(open(".github/workflows/ship-image.yml"))["jobs"]
out = []
if set(jobs) != {"image", "image-depot"}:
    out.append("jobs: %s" % sorted(jobs))
else:
    if "vars.TEQ_DEPOT_PROJECT == ''" not in jobs["image"]["if"] or "vars.TEQ_DEPOT_PROJECT != ''" not in jobs["image-depot"]["if"]:
        out.append("the two builds are not exclusive by TEQ_DEPOT_PROJECT")
    if "id-token" in jobs["image"]["permissions"] or jobs["image-depot"]["permissions"] != {"contents": "read", "id-token": "write", "packages": "write"}:
        out.append("permissions: %s, %s" % (jobs["image"]["permissions"], jobs["image-depot"]["permissions"]))
    build = [s_ for s_ in jobs["image-depot"]["steps"] if str(s_.get("uses", "")).startswith("depot/build-push-action@")]
    w_ = build[0]["with"] if build else {}
    if w_.get("project") != "${{ vars.TEQ_DEPOT_PROJECT }}" or w_.get("platforms") != "linux/amd64,linux/arm64" or w_.get("push") is not True \
            or w_.get("file") != "bench/actions/ship.Dockerfile" or w_.get("context") != "." or "steps.tag.outputs.tag" not in str(w_.get("tags")) \
            or "token" in w_:
        out.append("depot's build: %s" % w_)
    runs = [str(s_.get("run", "")) for s_ in jobs["image-depot"]["steps"]]
    if not any("image.sh login" in r for r in runs) or not runs[-1].endswith("bench/actions/image.sh record"):
        out.append("depot's build not logged in and recorded by image.sh")
print("\n".join(out))
PY
)
  check "ship-image.yml: Depot's builders by TEQ_DEPOT_PROJECT, buildx else" "$depot" ""
else
  echo "skip the workflow's structure: no python3 yaml"
fi

# --- bench/ship.sh --step, in a scratch repository whose builds are stand-ins writing the products.
s=$work/ship
mkdir -p "$s/bench/actions" "$s/tests/support" "$s/integrations/sbt/binary" "$s/src" "$work/bin" "$work/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin"
cp bench/ship.sh bench/ship-manifest.sh bench/ship-release.sh bench/ship-qualified.txt "$s/bench/"
printf '[package]\nname = "teq"\nversion = "0.1.7"\n' > "$s/Cargo.toml"
echo 'fn main() {}' > "$s/src/main.rs"
echo 'programs="a b"; left_out=""' > "$s/bench/programs.sh"
echo 'jars_warm() { :; }' > "$s/tests/support/jars.sh"
cat > "$s/bench/stub.sh" << 'EOF'
# The stand-ins' products: a file named for what made it.
make() { mkdir -p "$(dirname "$1")" && echo "$2" > "$1" && echo "${1##*/} $2" >> "$STUB_LOG"; }
need() { [ -e "$1" ] || { echo "stub: no $1"; exit 1; }; }
# binary <file> <word> [<training record>]: a binary and its manifest, of the tree's commit, the training's record in it.
binary() { make "$1" "binary $2"; { printf 'binary x\nversion teq 0.1.7 abc pgo\ncommit %s\n' "$(git rev-parse HEAD)"; [ -z "${3:-}" ] || cat "$3"; } > "$1.manifest"; echo "teq 0.1.7 abc pgo" > "$1.version"; }
training() { printf 'training status ok\ntraining programs 2\ntraining corpus c\ntraining jars j\ntraining profile p\ntraining release 0.1.7 %s\ntraining tuple image img-own\n' "$(git rev-parse HEAD)" > "$1"; }
EOF
cat > "$s/bench/cross-ship.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.." && . bench/stub.sh
case $1 in
  arm) make target/cross/arm.profdata arm; training target/cross/arm.training ;;
  use) need target/cross/arm.profdata; binary out/cross/teq-guided-arm use target/cross/arm.training ;;
  intel) need target/pgo/ship/teq.profdata; binary out/cross/teq-guided-intel intel target/pgo/ship/training.txt ;;
  linux-arm) need target/cross/arm.profdata; need out/tests/a.js; binary out/cross/teq-guided-linux-arm linux-arm target/cross/arm.training; make target/cross/guided-linux-arm.suite suite
    echo "suite x under qemu-user" >> out/cross/teq-guided-linux-arm.manifest ;;
  # The native route: the build's manifest names its bytes and no suite; the suite's record names the bytes it ran.
  linux-arm-build) need target/cross/arm.profdata; make out/cross/teq-guided-linux-arm linux-arm-build
    { printf 'binary %s\nversion teq 0.1.7 abc pgo\ncommit %s\n' "$(sha256sum < out/cross/teq-guided-linux-arm | cut -c1-64)" "$(git rev-parse HEAD)"; cat target/cross/arm.training; } > out/cross/teq-guided-linux-arm.manifest
    echo "teq 0.1.7 abc pgo" > out/cross/teq-guided-linux-arm.version ;;
  linux-arm-suite) need out/cross/teq-guided-linux-arm; need out/tests/a.js; need out/tests-jars.tgz; make target/cross/guided-linux-arm.suite suite
    printf "suite %s tests/run.sh passed: 1 passed, 0 failed, 0 of them skipped for missing jars, outputs 1 of 1 as the native suite's, natively in floor over deb\nsuite-tuple floor-aarch64 floor\n" \
      "${STUB_SUITE_SHA:-$(sha256sum < out/cross/teq-guided-linux-arm | cut -c1-64)}" > out/cross/teq-guided-linux-arm.suite ;;
  windows) binary out/cross/teq-windows-x86_64.exe windows; make target/cross/windows.smoke smoke ;;
esac
EOF
printf '#!/bin/bash\ncd "$(dirname "$0")/../.." && . bench/stub.sh\n[ "$1" != pack ] || make "$2" jars\n' > "$s/bench/actions/jars.sh"
printf '#!/bin/bash\n[ "$1" = write ] && mkdir -p "$3" && echo profiles > "$3/teq-0.1.7-profiles.tar"\n' > "$s/bench/ship-profiles.sh"
cat > "$s/bench/pgo.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.." && . bench/stub.sh
case $2 in
  gen) make target/pgo/ship/gen/ship/teq gen; make target/pgo/ship/linker.stamp stamp ;;
  train) need target/pgo/ship/gen/ship/teq; make target/pgo/ship/teq.profdata x86; training target/pgo/ship/training.txt
    make target/pgo/ship/corpus.txt corpus; make target/pgo/ship/jars.txt jars ;;
  use) need target/pgo/ship/teq.profdata; binary target/ship/teq use-x86 target/pgo/ship/training.txt; make target/pgo/ship/suite.log suite; make out/tests/a.js js ;;
esac
EOF
cat > "$s/integrations/sbt/binary/stage.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")" || exit 1
mkdir -p "binaries/$2" && cp "$1" "binaries/$2/teq" && { cat "$1.manifest"; echo "run $TEQ_SHIP_RUN"; } > "binaries/$2/teq.manifest"
EOF
printf '#!/bin/bash\necho "check.sh $*" >> "$STUB_LOG"\n' > "$s/integrations/sbt/binary/check.sh"
chmod +x "$s"/bench/*.sh "$s"/bench/actions/*.sh "$s"/integrations/sbt/binary/*.sh
for tool in rustup cargo dpkg wine node; do printf '#!/bin/sh\nexit 0\n' > "$work/bin/$tool"; done
printf '#!/bin/sh\ncase "$*" in -vV) printf "rustc 1.98.1\\nhost: x86_64-unknown-linux-gnu\\n" ;; "--print sysroot") echo %s ;; esac\n' "$work/sysroot" > "$work/bin/rustc"
printf '#!/bin/sh\nexit 0\n' > "$work/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"
chmod +x "$work/bin"/* "$work/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"
printf 'target/\nout/\nintegrations/sbt/binary/binaries/\n' > "$s/.gitignore"
(cd "$s" && git_ init -q && git_ add -A && git_ commit -q -m base) || exit 1
ship() { (cd "$s" && PATH=$work/bin:$PATH STUB_LOG=$work/stub.log timeout 120 bench/ship.sh "$@") > "$work/ship.out" 2>&1; }
: > "$work/stub.log"
ship --step cross-arm --carry "$work/c1"
check "ship --step cross-arm" "$?" 0
check "ship --step cross-arm: its products alone" "$(cd "$work/c1" && find . -type f | LC_ALL=C sort | tr '\n' ' ')" "./target/cross/arm.profdata ./target/cross/arm.training "
ship --step cross-use --carry "$work/c1"
check "ship --step cross-use from the carried profile" "$?" 0
check "ship --step cross-use: the Darwin binary carried" "$(cat "$work/c1/out/cross/teq-guided-arm" 2> /dev/null)" "binary use"
ship --step cross-intel --carry "$work/c-empty"
check "ship --step cross-intel without the x86 profile fails" "$?" 1
: > "$work/stub.log"
ship --step pgo-use,pgo-train,pgo-gen --carry "$work/c2"
check "ship --step pgo-*" "$?" 0
check "ship --step: the ship's order whatever the order named" "$(awk '{print $1}' "$work/stub.log" | tr '\n' ' ')" "teq linker.stamp teq.profdata corpus.txt jars.txt teq suite.log a.js tests-jars.tgz "
check "ship --step pgo-use: the jars its suite built carried" "$([ -f "$work/c2/out/tests-jars.tgz" ] && echo carried)" carried
ship --step cross-intel --carry "$work/c2"
check "ship --step cross-intel from the carried x86 profile" "$?" 0
cp -a "$work/c1/." "$work/c2/"
ship --step cross-linux-arm,cross-windows --carry "$work/c2"
check "ship --step cross-linux-arm,cross-windows" "$?" 0
: > "$work/stub.log"
ship --step stage --carry "$work/c2"
check "ship --step stage" "$?" 0
check "ship --step stage: the five staged into the carry, held back as the file holds them" \
  "$(cd "$work/c2" && find integrations/sbt/binary/binaries out/ship/held -name teq | LC_ALL=C sort | tr '\n' ' ')" \
  "out/ship/held/linux-aarch_64/teq out/ship/held/linux-x86_64/teq out/ship/held/osx-aarch_64/teq out/ship/held/osx-x86_64/teq out/ship/held/windows-x86_64/teq "
check "ship --step stage: one staging invocation" "$(cat "$work"/c2/out/ship/held/*/teq.manifest | grep -c '^run ship-')" 5
check "ship --step stage: the profiles' asset beside the five" "$(ls "$work/c2/out/ship/profiles" 2> /dev/null)" teq-0.1.7-profiles.tar
check "ship --step stage: the run is one" "$(cat "$work"/c2/out/ship/held/*/teq.manifest | sed -n 's/^run //p' | sort -u | wc -l | tr -d ' ')" 1
has "ship --step stage: says none is qualified" "$work/ship.out" "no classifier qualified, none to check"
# A dry run on the last release's profiles, taken up after a program joined the corpus while master still names that
# release (one version for both): the builds' manifests name the profiles' source, and the stage takes the profiles'
# own corpus of 1 program, where the tree's has 2; a training of the ship's own of 1 program is refused as before.
# taken <carry> <source line or nothing> <the commit trained>: the two records of a training of 1 program.
taken() {
  local f
  for f in target/pgo/ship/training.txt target/cross/arm.training; do
    { printf 'training status ok\ntraining programs 1\ntraining corpus c\ntraining jars j\ntraining profile p\ntraining release 0.1.7 %s\ntraining tuple image img-earlier\n' "$3"
      [ -z "$2" ] || echo "$2"; } > "$1/$f"
  done
}
rm -rf -- "$work/r1" && cp -a "$work/c2" "$work/r1" && rm -rf -- "$work/r1/integrations" "$work/r1/out/ship"
taken "$work/r1" "training source v0.1.7 $(printf '%064d' 2)" "$(printf '%040d' 1)"
ship --step cross-use,pgo-use,cross-intel,cross-linux-arm --carry "$work/r1"
check "a dry run on the last release's profiles: the builds" "$?" 0
ship --step stage --carry "$work/r1"
check "a dry run on the last release's profiles: the stage takes their corpus, not the tree's grown one" "$?" 0
has "a dry run on the last release's profiles: says whose they are" "$work/ship.out" "taken from v0.1.7, on its corpus of 1 programs (2 in this tree)"
check "a dry run on the last release's profiles: the four guided manifests name their source" "$(cat "$work"/r1/out/ship/held/*/teq.manifest | grep -c '^training source v0.1.7 ')" 4
rm -rf -- "$work/r2" && cp -a "$work/r1" "$work/r2" && rm -rf -- "$work/r2/integrations" "$work/r2/out/ship"
taken "$work/r2" "" "$(git -C "$s" rev-parse HEAD)"
ship --step stage --carry "$work/r2"
check "a training of the ship's own of 1 program in a tree of 2: refused" "$?" 1
has "a training of the ship's own: says so" "$work/ship.out" "the training ran 1 programs, not 2"
ship --step nothing --carry "$work/c3"
check "ship --step of no step" "$?" 2
ship --publish --step cross-arm --carry "$work/c3"
check "ship --publish --step" "$?" 2
ship --step cross-arm
check "ship --step without --carry" "$?" 2
if [ -x /usr/bin/time ]; then
  (cd "$s" && cp "$root/bench/actions/step.sh" bench/ && mkdir -p bench/actions && mv bench/step.sh bench/actions/ && git_ add -A && git_ commit -q -m step) || exit 1
  (cd "$s" && PATH=$work/bin:$PATH STUB_LOG=$work/stub.log GITHUB_STEP_SUMMARY=$work/step.summary timeout 120 bench/actions/step.sh cross-use "$work/c4" "$work/c4.tar" "$work/c1.tar") > "$work/step.out" 2>&1
  check "step.sh without the tar of the profile fails" "$?" 1
  tar -cf "$work/c1.tar" -C "$work/c1" target/cross/arm.profdata target/cross/arm.training
  rm -rf -- "$work/c4"
  (cd "$s" && PATH=$work/bin:$PATH STUB_LOG=$work/stub.log GITHUB_STEP_SUMMARY=$work/step.summary timeout 120 bench/actions/step.sh cross-use "$work/c4" "$work/c4.tar" "$work/c1.tar") > "$work/step.out" 2>&1
  check "step.sh cross-use" "$?" 0
  check "step.sh packs the step's products alone" "$(tar -tf "$work/c4.tar" | LC_ALL=C sort | tr '\n' ' ')" "./out/cross/teq-guided-arm ./out/cross/teq-guided-arm.manifest ./out/cross/teq-guided-arm.version "
  has "step.sh writes the measures" "$work/step.summary" "Peak resident memory"
  # The stage's summary lists the environment of a training of the ship's own among the lines to qualify, never that
  # of profiles taken from an earlier release (the dry run above).
  for r in c2 r1; do
    tar -cf "$work/$r.in.tar" -C "$work/$r" target out/cross out/tests out/tests-jars.tgz
    rm -rf -- "$work/$r.stage"
    (cd "$s" && PATH=$work/bin:$PATH STUB_LOG=$work/stub.log GITHUB_STEP_SUMMARY=$work/$r.summary timeout 120 bench/actions/step.sh stage "$work/$r.stage" "$work/$r.staged.tar" "$work/$r.in.tar") > "$work/step.out" 2>&1
    check "step.sh stage ($r)" "$?" 0
  done
  has "step.sh stage: an own training's environment among the lines to qualify" "$work/c2.summary" "    image img-own"
  lacks "step.sh stage: a taken training's environment not among them" "$work/r1.summary" "img-earlier"
else
  echo "skip bench/actions/step.sh: no /usr/bin/time"
fi

# --- The native route's steps, as an arm64 Linux machine runs them (uname's and rustc's stand-ins say aarch64,
# never a real aarch64 run): its two steps admitted with their own tools (no wine, qemu-user or Rust for the
# suite), the others refused, the suite refused on x86-64; linux-arm-build's binary and linux-arm-suite's record
# staged together, a record of other bytes refused; one route per run.
mkdir -p "$work/bin-arm" "$work/sysroot-arm/lib/rustlib/aarch64-unknown-linux-gnu/bin"
uname_real=$(command -v uname)
printf '#!/bin/sh\ncase "$1" in -m) echo aarch64 ;; -s) echo Linux ;; *) exec %s "$@" ;; esac\n' "$uname_real" > "$work/bin-arm/uname"
printf '#!/bin/sh\ncase "$*" in -vV) printf "rustc 1.98.1\\nhost: aarch64-unknown-linux-gnu\\n" ;; "--print sysroot") echo %s ;; esac\n' "$work/sysroot-arm" > "$work/bin-arm/rustc"
for tool in rustup cargo docker; do printf '#!/bin/sh\nexit 0\n' > "$work/bin-arm/$tool"; done
printf '#!/bin/sh\nexit 0\n' > "$work/sysroot-arm/lib/rustlib/aarch64-unknown-linux-gnu/bin/llvm-profdata"
chmod +x "$work/bin-arm"/* "$work/sysroot-arm/lib/rustlib/aarch64-unknown-linux-gnu/bin/llvm-profdata"
ship_arm() { (cd "$s" && PATH=$work/bin-arm:$PATH STUB_LOG=$work/stub.log timeout 120 bench/ship.sh "$@") > "$work/ship.out" 2>&1; }
ship_arm --step cross-arm --carry "$work/a1"
check "arm64: ship --step cross-arm, wine and qemu-user not asked for" "$?/$(cd "$work/a1" && find . -type f | LC_ALL=C sort | tr '\n' ' ')" "0/./target/cross/arm.profdata ./target/cross/arm.training "
ship_arm --step cross-use --carry "$work/a1"
check "arm64: ship --step cross-use refused" "$?" 1
has "arm64: says what an arm64 machine runs" "$work/ship.out" "an arm64 Linux machine runs cross-arm and linux-arm-suite alone"
ship_arm
check "arm64: the whole ship refused" "$?" 1
ship --step linux-arm-suite --carry "$work/c2"
check "x86-64: ship --step linux-arm-suite refused" "$?" 1
ship --step cross-linux-arm,linux-arm-build --carry "$work/c2"
check "ship --step of both Linux aarch64 routes refused" "$?" 2
cp -a "$work/c2" "$work/n1" && rm -rf "$work/n1/out/cross/teq-guided-linux-arm"* "$work/n1/target/cross/guided-linux-arm.suite" "$work/n1/integrations" "$work/n1/out/ship"
ship --step linux-arm-build --carry "$work/n1"
check "ship --step linux-arm-build" "$?" 0
check "linux-arm-build: a manifest without a suite" "$(grep -c '^suite' "$work/n1/out/cross/teq-guided-linux-arm.manifest")" 0
ship --step stage --carry "$work/n1"
check "ship --step stage without the native suite's record refused" "$?" 1
(cd "$s" && PATH=$work/bin-arm:$PATH STUB_LOG=$work/stub.log STUB_SUITE_SHA=$(printf '%064d' 3) timeout 120 bench/ship.sh --step linux-arm-suite --carry "$work/n1") > "$work/ship.out" 2>&1
check "arm64: ship --step linux-arm-suite, no Rust asked for" "$?" 0
ship --step stage --carry "$work/n1"
check "ship --step stage with a suite record of other bytes refused" "$?" 1
has "stage: says the record is not of the binary" "$work/ship.out" "is not one suite of"
ship_arm --step linux-arm-suite --carry "$work/n1"
check "arm64: ship --step linux-arm-suite again" "$?" 0
ship --step stage --carry "$work/n1"
check "ship --step stage with the native suite's record" "$?" 0
check "stage: the record in the staged manifest" "$(grep -c -e '^suite [0-9a-f]* tests/run.sh passed: .* natively in floor over deb$' -e '^suite-tuple floor-aarch64 floor$' "$work/n1/out/ship/held/linux-aarch_64/teq.manifest")" 2
if [ -x /usr/bin/time ]; then
  # step.sh as the workflow's arm64 job runs it: the carried tars in, the products of its step alone out.
  rm -rf -- "$work/a2" && tar -cf "$work/n1.tar" -C "$work/n1" out/cross target/cross out/tests out/tests-jars.tgz
  (cd "$s" && PATH=$work/bin-arm:$PATH STUB_LOG=$work/stub.log GITHUB_STEP_SUMMARY=$work/step-arm.summary timeout 120 bench/actions/step.sh linux-arm-suite "$work/a2" "$work/a2.tar" "$work/n1.tar") > "$work/step.out" 2>&1
  check "arm64: step.sh linux-arm-suite" "$?" 0
  check "arm64: step.sh packs the suite's record and log alone" "$(tar -tf "$work/a2.tar" | LC_ALL=C sort | tr '\n' ' ')" "./out/cross/teq-guided-linux-arm.suite ./target/cross/guided-linux-arm.suite "
  has "arm64: step.sh's summary names the machine" "$work/step-arm.summary" "aarch64."
  rm -rf -- "$work/a3"
  (cd "$s" && PATH=$work/bin-arm:$PATH STUB_LOG=$work/stub.log GITHUB_STEP_SUMMARY=$work/step-arm.summary timeout 120 bench/actions/step.sh cross-arm "$work/a3" "$work/a3.tar") > "$work/step.out" 2>&1
  check "arm64: step.sh cross-arm, its profile and record carried" "$?/$(tar -tf "$work/a3.tar" | LC_ALL=C sort | tr '\n' ' ')" "0/./target/cross/arm.profdata ./target/cross/arm.training "
fi

# --- bench/cross-ship.sh itself, its machine forced by stand-ins (uname's, rustc's and cargo's; never a real aarch64
# run or build): the aarch64 trainer's training natively on arm64 (no qemu-user fetched, the trainer run as it is,
# its record native with the metadata and the machine's toolchain) and under qemu-user on x86-64 as before (the
# wrapper, the give-ways, the qemu-user named); the stages each machine refuses; the native suite in the floor's
# container (docker's stand-in runs the container's script here, its dpkg, dpkg-query and setpriv stand-ins
# too), its record, and its refusals.
x=$work/xs xbin=$work/xbin
mkdir -p "$x/bench/actions" "$x/tests/support" "$x/tests/cases" "$xbin" "$work/xfloor" \
  "$work/xsys/lib/rustlib/aarch64-unknown-linux-gnu/bin" "$work/xsys/lib/rustlib/x86_64-unknown-linux-gnu/bin"
cp bench/cross-ship.sh bench/ship-manifest.sh bench/ship-metadata.sh bench/ship-release.sh bench/zig.sh "$x/bench/"
printf '[package]\nname = "teq"\nversion = "0.1.7"\n' > "$x/Cargo.toml"
echo '[toolchain]' > "$x/rust-toolchain.toml"
echo 'object A' > "$x/tests/cases/end_markers.scala"
echo ':' > "$x/tests/support/jars.sh"
printf '#!/bin/bash\n[ "$1" = restore ] && [ -f "$2" ]\n' > "$x/bench/actions/jars.sh"
# The training's stand-in: one run of the instrumented binary in its place (the trainer, or qemu_wrapper's script).
cat > "$x/bench/pgo.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.." || exit 1
. bench/ship-manifest.sh
d=target/pgo/ship
"$d/gen/ship/teq" compiler check x > /dev/null || { echo "pgo: training run failed"; exit 1; }
echo counts > "$d/teq.profdata"
printf 'training status ok\ntraining programs 2\ntraining runs 1\ntraining corpus c 1 files\ntraining jars j 1 jars\ntraining profile %s\ntraining trainer x native\ntraining tuple rustc of-pgo\n' "$(manifest_sha256 "$d/teq.profdata")" > "$d/training.txt"
EOF
# The suite's stand-in: the native suite's outputs written again, one other with STUB_SUITE_DIFF.
cat > "$x/tests/run.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.." || exit 1
mkdir -p "$TEQ_TEST_OUT" && cp out/tests/* "$TEQ_TEST_OUT/" || exit 1
[ -z "$STUB_SUITE_DIFF" ] || echo other > "$TEQ_TEST_OUT/a.js"
echo "1 passed, 0 failed"
EOF
chmod +x "$x/bench/"*.sh "$x/bench/actions/jars.sh" "$x/tests/run.sh"
(cd "$x" && git_ init -q && git_ add -A && git_ commit -q -m cross) || exit 1
cat > "$xbin/uname" << EOF
#!/bin/sh
case "\$1" in -m) echo "\$STUB_ARCH" ;; -s) echo Linux ;; *) exec $uname_real "\$@" ;; esac
EOF
cat > "$xbin/rustc" << EOF
#!/bin/sh
[ -z "\$STUB_NO_RUST" ] || { echo "rustc \$*" >> "\$STUB_LOG"; exit 1; }
case "\$*" in
  -vV) printf 'rustc 1.98.1\nbinary: rustc\ncommit-hash: c1\nhost: %s\nrelease: 1.98.1\nLLVM version: 22.1.8\n' "\$STUB_ARCH-unknown-linux-gnu" ;;
  "--print sysroot") echo $work/xsys ;;
  *) exit 1 ;;
esac
EOF
cat > "$xbin/cargo" << 'EOF'
#!/bin/bash
[ "$1" = -vV ] && { printf 'cargo 1.98.1\nrelease: 1.98.1\ncommit-hash: c4\n'; exit 0; }
[ -n "$TEQ_CROSS_PROBE" ] && { "$RUSTC_WRAPPER" rustc --crate-name teq -C metadata=0123abcd; exit 1; }
target= dir= prev=
for a in "$@"; do [ "$prev" = --target ] && target=$a; [ "$prev" = --target-dir ] && dir=$a; prev=$a; done
mkdir -p "$dir/$target/ship" || exit 1
printf '#!/bin/sh\n[ "$1" = --version ] && { echo "teq 0.1.7 abc instrumented"; exit 0; }\necho "trainer $*" >> %s\n' "$STUB_LOG" > "$dir/$target/ship/teq"
chmod +x "$dir/$target/ship/teq" && echo Finished
EOF
printf '#!/bin/sh\n[ "$*" = "target list --installed" ] && echo aarch64-unknown-linux-musl\nexit 0\n' > "$xbin/rustup"
printf '#!/bin/sh\necho "curl $*" >> "$STUB_LOG"\nexit 22\n' > "$xbin/curl"
printf '#!/bin/sh\n[ "$1" = --version ] && { echo "qemu-aarch64 version 10.2.1"; exit 0; }\necho qemu >> "$STUB_LOG"\n[ "$1" != -L ] || shift 2\nexec "$@"\n' > "$xbin/qemu-aarch64"
cat > "$xbin/docker" << 'EOF'
#!/bin/bash
echo "docker $1" >> "$STUB_LOG"
case $1 in pull | rm) exit 0 ;; run) shift ;; *) exit 2 ;; esac
envs=()
while [ $# -gt 0 ]; do
  case $1 in
    -e) envs+=("$2"); shift 2 ;;
    -v | -w | --name | --platform | --network) shift 2 ;;
    --rm | --init) shift ;;
    *) break ;;
  esac
done
echo "docker image $1" >> "$STUB_LOG"
shift
exec env PATH="$STUB_FLOOR_BIN:$PATH" "${envs[@]}" "$@"
EOF
printf '#!/bin/sh\necho "dpkg $*" >> "$STUB_LOG"\nexit 0\n' > "$work/xfloor/dpkg"
printf '#!/bin/sh\nprintf %%s "${STUB_FLOOR_LIBC:-2.28-10+deb10u1}"\n' > "$work/xfloor/dpkg-query"
printf '#!/bin/sh\nwhile [ "$1" != --clear-groups ]; do shift; done\nshift\nexec "$@"\n' > "$work/xfloor/setpriv"
for d in aarch64 x86_64; do printf '#!/bin/sh\nexit 0\n' > "$work/xsys/lib/rustlib/$d-unknown-linux-gnu/bin/llvm-profdata"; done
chmod +x "$xbin"/* "$work/xfloor"/* "$work/xsys"/lib/rustlib/*/bin/*
cross() { (cd "$x" && env PATH="$xbin:$PATH" STUB_LOG="$work/x.log" STUB_FLOOR_BIN="$work/xfloor" "$@") > "$work/x.out" 2>&1; }
: > "$work/x.log"
cross STUB_ARCH=aarch64 TEQ_SHIP_IMAGE=img-arm timeout 120 bench/cross-ship.sh arm
check "cross-ship arm on arm64: native" "$?/$(manifest_get() { sed -n "s/^$2 //p" "$1"; }; manifest_get "$x/target/cross/arm.training" "training trainer")" "0/aarch64-unknown-linux-musl native"
check "cross-ship arm on arm64: no qemu-user fetched or run, the trainer run as it is" "$(grep -c -e '^curl' -e '^qemu' "$work/x.log")/$(grep -c '^trainer compiler check x' "$work/x.log")" "0/1"
check "cross-ship arm on arm64: its record" "$(grep -E '^training (metadata|host|release|give-ways|tuple (image|image-aarch64|qemu-user|rustc)) ' "$x/target/cross/arm.training" | sed 's/ [0-9a-f]\{40\}$/ <commit>/' | tr '\n' '|')" \
  "training release 0.1.7 <commit>|training metadata 0123abcd|training host aarch64-unknown-linux-gnu|training tuple rustc 1.98.1 c1|training tuple image-aarch64 img-arm|"
: > "$work/x.log"
cross STUB_ARCH=x86_64 TEQ_SHIP_IMAGE=img TEQ_CROSS_QEMU="$xbin/qemu-aarch64" timeout 120 bench/cross-ship.sh arm
qsha=$(sha256sum < "$xbin/qemu-aarch64" | cut -c1-64)
check "cross-ship arm on x86-64: under qemu-user, as before" "$?/$(sed -n 's/^training trainer //p' "$x/target/cross/arm.training")/$(grep -c '^qemu' "$work/x.log")" "0/aarch64-unknown-linux-musl under qemu-user named $qsha/1"
check "cross-ship arm on x86-64: its record" "$(grep -E '^training (host|give-ways|tuple (image|image-aarch64|qemu-user)) ' "$x/target/cross/arm.training" | tr '\n' '|')" \
  "training host x86_64-unknown-linux-gnu|training give-ways 0|training tuple qemu-user named $qsha|training tuple image img|"
# zig's cache where the caller names it, outside the tree the ship replaces at each run; the tree's without.
zig_cache_of() { (cd "$x" && env PATH="$xbin:$PATH" STUB_ARCH=x86_64 "$@" bash -c '. bench/zig.sh && zig=/zig && zig_wrapper aarch64-unknown-linux-gnu > /dev/null && sed -n "s/^export ZIG_GLOBAL_CACHE_DIR=\([^ ]*\) .*/\1/p" "$zig_cc"'); }
check "zig's cache: TEQ_ZIG_CACHE's" "$(zig_cache_of TEQ_ZIG_CACHE=/github/home/.cache/zig)" /github/home/.cache/zig
check "zig's cache: the tree's without" "$(zig_cache_of)" "$x/target/cross/zig-cache"
cross STUB_ARCH=aarch64 timeout 120 bench/cross-ship.sh use
check "cross-ship use on arm64: refused" "$?" 1
has "cross-ship use on arm64: says what arm64 makes" "$work/x.out" "an arm64 Linux machine makes arm and linux-arm-suite alone"
cross STUB_ARCH=x86_64 timeout 120 bench/cross-ship.sh linux-arm-suite
check "cross-ship linux-arm-suite on x86-64: refused" "$?" 1
# The native suite, against fixtures in the place of the floor's package and node (the script's pins moved to
# their digests: the floor's real files are network's).
mkdir -p "$x/target/cross" "$x/out/cross" "$x/out/tests" "$work/xnode/node-v24.21.0-linux-arm64/bin"
echo deb > "$x/target/cross/libc6_2.28-10+deb10u1_arm64.deb"
printf '#!/bin/sh\necho v24.21.0\n' > "$work/xnode/node-v24.21.0-linux-arm64/bin/node" && chmod +x "$work/xnode/node-v24.21.0-linux-arm64/bin/node"
tar -cJf "$x/target/cross/node-v24.21.0-linux-arm64.tar.xz" -C "$work/xnode" node-v24.21.0-linux-arm64
debsha=$(sha256sum < "$x/target/cross/libc6_2.28-10+deb10u1_arm64.deb" | cut -c1-64)
sed -e "s/^sysroot_sha256=.*/sysroot_sha256=$debsha/" -e "s/^floor_node_sha256=.*/floor_node_sha256=$(sha256sum < "$x/target/cross/node-v24.21.0-linux-arm64.tar.xz" | cut -c1-64)/" bench/cross-ship.sh > "$x/bench/cross-ship.sh"
printf '#!/bin/sh\necho "teq 0.1.7 abc pgo"\n' > "$x/out/cross/teq-guided-linux-arm" && chmod +x "$x/out/cross/teq-guided-linux-arm"
lsha=$(sha256sum < "$x/out/cross/teq-guided-linux-arm" | cut -c1-64)
printf 'binary %s\nversion teq 0.1.7 abc pgo\n' "$lsha" > "$x/out/cross/teq-guided-linux-arm.manifest"
echo js > "$x/out/tests/a.js" && echo jars > "$x/out/tests-jars.tgz"
: > "$work/x.log"
cross STUB_ARCH=aarch64 STUB_NO_RUST=1 timeout 120 bench/cross-ship.sh linux-arm-suite
check "cross-ship linux-arm-suite on arm64" "$?" 0
check "linux-arm-suite: no Rust asked, the floor's image run, its package installed" \
  "$(grep -c '^rustc' "$work/x.log")/$(sed -n 's/^docker image //p' "$work/x.log")/$(grep -c '^dpkg -i .*libc6_2.28-10+deb10u1_arm64.deb$' "$work/x.log")" \
  "0/docker.io/library/debian@sha256:fba020fe61e2b15959ef887ea67c7fc61f77943d074889debe8d92e29402191f/1"
check "linux-arm-suite: its record" "$(cat "$x/out/cross/teq-guided-linux-arm.suite" 2> /dev/null | tr '\n' '|')" \
  "suite $lsha tests/run.sh passed: 1 passed, 0 failed, 0 of them skipped for missing jars, outputs 1 of 1 as the native suite's, natively in docker.io/library/debian@sha256:fba020fe61e2b15959ef887ea67c7fc61f77943d074889debe8d92e29402191f over libc6_2.28-10+deb10u1_arm64.deb $debsha|suite-tuple floor-aarch64 docker.io/library/debian@sha256:fba020fe61e2b15959ef887ea67c7fc61f77943d074889debe8d92e29402191f|suite-tuple sysroot-aarch64 libc6_2.28-10+deb10u1_arm64.deb $debsha|suite-tuple node-aarch64 24.21.0 $(sed -n 's/^floor_node_sha256=//p' "$x/bench/cross-ship.sh")|"
cross STUB_ARCH=aarch64 STUB_SUITE_DIFF=1 timeout 120 bench/cross-ship.sh linux-arm-suite
check "linux-arm-suite with an output not the native suite's: refused, no record" "$?/$([ -e "$x/out/cross/teq-guided-linux-arm.suite" ] && echo record)" "1/"
cross STUB_ARCH=aarch64 STUB_FLOOR_LIBC=2.28-10+deb10u3 timeout 120 bench/cross-ship.sh linux-arm-suite
check "linux-arm-suite over another libc6 than the floor's package: refused" "$?" 1
has "linux-arm-suite: names the libc6" "$work/x.out" "libc6 is 2.28-10+deb10u3"
printf 'binary %s\nversion teq 0.1.7 abc pgo\n' "$(printf '%064d' 4)" > "$x/out/cross/teq-guided-linux-arm.manifest"
cross STUB_ARCH=aarch64 timeout 120 bench/cross-ship.sh linux-arm-suite
check "linux-arm-suite of a binary not its manifest's: refused" "$?" 1

# --- The manifests' records of the two routes, by bench/ship-manifest.sh's own checks (the binary's header, floor and
# version stood in for): the trainer native or under qemu-user, the Linux aarch64 suite natively in the floor's
# container or under qemu-user, each against the qualified tools; the environment of a training of the release's
# own qualified whole, an earlier release's by its compiler.
m=$work/manifests
mkdir -p "$m"
head_=$(git rev-parse HEAD)
printf 'rustc 1.98.1 c1\nllvm 22.1.8\ncargo 1.98.1 c4\nos Ubuntu 26.04 resolute\nzig 0.16.0 z\nqemu-user 10.2.1 q\nsysroot-aarch64 libc6.deb s\nfloor-aarch64 floor@sha256:f\nnode-aarch64 24.21.0 n\nimage img\nimage-aarch64 img-arm\nroute osx-aarch_64\nroute linux-aarch_64\n' > "$m/qualified"
grep -v '^qemu-user ' "$m/qualified" > "$m/qualified-no-qemu"
echo bytes > "$m/teq" && msha=$(sha256sum < "$m/teq" | cut -c1-64)
# manifest <file> <target> <line>...: a manifest of the binary built from the head as 0.1.7, its toolchain the qualified one.
manifest() {
  local f=$1 t=$2
  shift 2
  { printf 'binary %s\nversion teq 0.1.7 %s pgo\ncommit %s\ntarget %s\ntraining status ok\n' "$msha" "${head_:0:9}" "$head_" "$t"
    printf 'tuple rustc 1.98.1 c1\ntuple llvm 22.1.8\ntuple cargo 1.98.1 c4\ntuple os Ubuntu 26.04 resolute\ntuple zig 0.16.0 z\ntuple image img\n'
    printf '%s\n' "$@"; } > "$m/$f"
}
own="training release 0.1.7 $head_"
native_env=("training trainer aarch64-unknown-linux-musl native" "training tuple rustc 1.98.1 c1" "training tuple llvm 22.1.8" "training tuple cargo 1.98.1 c4" "training tuple os Ubuntu 26.04 resolute")
qualified() { (. bench/ship-manifest.sh && manifest_qualified "$m/$1" "$m/$2" "$3") > "$m/out" 2>&1; echo $?; }
manifest mac-native aarch64-apple-darwin "$own" "${native_env[@]}" "training tuple image-aarch64 img-arm"
check "qualified: a Darwin arm64 binary of a native training, no qemu-user named" "$(qualified mac-native qualified-no-qemu osx-aarch_64)" 0
manifest mac-native-other aarch64-apple-darwin "$own" "${native_env[@]}" "training tuple image-aarch64 other"
check "qualified: its own training in an arm64 image not qualified" "$(qualified mac-native-other qualified osx-aarch_64)" 1
manifest mac-earlier aarch64-apple-darwin "training release 0.1.6 $(printf '%040d' 3)" "${native_env[@]}" "training tuple image-aarch64 other"
check "qualified: an earlier release's profile, its compiler the qualified one" "$(qualified mac-earlier qualified osx-aarch_64)" 0
manifest mac-taken-same aarch64-apple-darwin "training release 0.1.7 $(printf '%040d' 1)" "training source v0.1.7 $(printf '%064d' 2)" "${native_env[@]}" "training tuple image-aarch64 other"
check "qualified: profiles taken from a release of the same version (a dry run on master before its bump)" "$(qualified mac-taken-same qualified osx-aarch_64)" 0
manifest mac-earlier-rustc aarch64-apple-darwin "training release 0.1.6 $(printf '%040d' 3)" "training trainer aarch64-unknown-linux-musl native" "training tuple rustc 1.97.0 c0"
check "qualified: an earlier release's profile of another compiler" "$(qualified mac-earlier-rustc qualified osx-aarch_64)" 1
manifest mac-qemu aarch64-apple-darwin "$own" "training trainer aarch64-unknown-linux-musl under qemu-user 10.2.1 q" "training tuple qemu-user 10.2.1 q"
check "qualified: a training under qemu-user, the file naming it" "$(qualified mac-qemu qualified osx-aarch_64)" 0
check "qualified: a training under qemu-user, the file not naming it" "$(qualified mac-qemu qualified-no-qemu osx-aarch_64)" 1
manifest mac-qemu-earlier aarch64-apple-darwin "training release 0.1.6 $(printf '%040d' 3)" "training trainer aarch64-unknown-linux-musl under qemu-user 9.0 old" "training tuple qemu-user 9.0 old" "training tuple rustc 1.98.1 c1"
check "qualified: an earlier release's profile trained under a qemu-user the file no longer names" "$(qualified mac-qemu-earlier qualified-no-qemu osx-aarch_64)" 0
native_suite=("glibc 2.28 needs libc.so.6" "suite $msha tests/run.sh passed: 3 passed, 0 failed, 0 of them skipped for missing jars, outputs 3 of 3 as the native suite's, natively in floor@sha256:f over libc6.deb s"
  "suite-tuple floor-aarch64 floor@sha256:f" "suite-tuple sysroot-aarch64 libc6.deb s" "suite-tuple node-aarch64 24.21.0 n")
manifest linux-native aarch64-unknown-linux-gnu "$own" "${native_env[@]}" "training tuple image-aarch64 img-arm" "${native_suite[@]}"
check "qualified: a Linux aarch64 binary of the native route" "$(qualified linux-native qualified-no-qemu linux-aarch_64)" 0
sed 's/^suite-tuple floor-aarch64 .*/suite-tuple floor-aarch64 floor@sha256:other/' "$m/linux-native" > "$m/linux-native-floor"
check "qualified: a native suite in another floor" "$(qualified linux-native-floor qualified linux-aarch_64)" 1
emulated_suite=("glibc 2.28 needs libc.so.6" "tuple qemu-user 10.2.1 q" "tuple sysroot-aarch64 libc6.deb s"
  "suite $msha tests/run.sh passed: 3 passed, 0 failed, 0 of them skipped for missing jars, outputs 3 of 3 as the native suite's, under qemu-user 10.2.1 q sysroot libc6.deb s")
manifest linux-emulated aarch64-unknown-linux-gnu "$own" "training trainer aarch64-unknown-linux-musl under qemu-user 10.2.1 q" "training tuple qemu-user 10.2.1 q" "${emulated_suite[@]}"
check "qualified: a Linux aarch64 binary of the emulated route, as before" "$(qualified linux-emulated qualified linux-aarch_64)" 0
# manifest_check, the binary's own readers stood in for.
checked() {
  (. bench/ship-manifest.sh && manifest_header() { echo linux-aarch_64; } && manifest_glibc() { echo "2.28 needs libc.so.6"; } && manifest_carries() { :; } &&
    manifest_check "$m/teq" "$m/$1" linux-aarch_64 "$head_") > "$m/out" 2>&1; echo $?
}
check "manifest_check: the native suite" "$(checked linux-native)" 0
check "manifest_check: a native suite in another floor than its record's" "$(checked linux-native-floor)" 1
check "manifest_check: the emulated suite" "$(checked linux-emulated)" 0
sed 's/^tuple qemu-user .*/tuple qemu-user 9.0 other/' "$m/linux-emulated" > "$m/linux-emulated-qemu"
check "manifest_check: an emulated suite under another qemu-user than the toolchain's" "$(checked linux-emulated-qemu)" 1
grep -v '^suite ' "$m/linux-native" > "$m/linux-built"
printf 'suite %s tests/run.sh passed: 3 passed, 0 failed, 0 of them skipped for missing jars, outputs 3 of 3 as the native suite'"'"'s, natively in floor@sha256:f over libc6.deb s\n' "$msha" > "$m/record"
cp "$m/teq" "$m/b" && grep -v '^suite' "$m/linux-native" > "$m/b.manifest"
check "manifest_attest: the record given to the manifest" "$( (. bench/ship-manifest.sh && manifest_attest "$m/b" "$m/record") > /dev/null 2>&1; echo $?)/$(grep -c '^suite ' "$m/b.manifest")" "0/1"
check "manifest_attest: a second record refused" "$( (. bench/ship-manifest.sh && manifest_attest "$m/b" "$m/record") > /dev/null 2>&1; echo $?)" 1

# --- tests/support/identity.sh across machines, with a stand-in compiler: its outputs a digest of the source,
# STANDIN_DIFF=<program> another for that one, STANDIN_KILL=<program> a run that dies of a signal.
cat > "$work/teq-standin" << 'EOF'
#!/bin/bash
[ "$1" = --version ] && { echo "${STANDIN_VERSION:-teq 9.9.9 $(git rev-parse --short HEAD) pgo}"; exit 0; }
[ "$1" = compiler ] || exit 2
shift
[ "$1" = --help ] && exit 0
verb=$1 src=$2 out=
shift 2
while [ $# -gt 0 ]; do [ "$1" = -o ] && out=$2; shift; done
name=$(basename "$src" .scala)
[ "$name" != "${STANDIN_KILL:-}" ] || kill -KILL $$
# The worker running the program (timeout's parent), killed: the program left without a result.
[ "$name" != "${STANDIN_KILL_WORKER:-}" ] || kill -KILL "$(ps -o ppid= -p $PPID | tr -d ' ')"
sum=$(cat "$src" "$src"/*.scala 2> /dev/null | sha256sum | cut -c1-16)
[ "$name" != "${STANDIN_DIFF:-}" ] || sum=other
case $verb in
  build)
    case $out in
      *.jar) python3 -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.writestr("A.class", sys.argv[2]); z.close()' "$out" "$sum" ;;
      *) echo "// $sum" > "$out" ;;
    esac
    echo "built $src in 3 ms" ;;
  check) echo "checked $src in 4 ms"; echo "$src: $sum"; [ "${src#tests/errors/}" = "$src" ] || exit 1 ;;
esac
EOF
chmod +x "$work/teq-standin"
cache=$work/cache-identity
mkdir -p "$cache/https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4"
python3 -c 'import zipfile, sys; zipfile.ZipFile(sys.argv[1], "w").writestr("scala/Predef.class", "p")' "$cache/https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
plain=$(cd tests/cases && for f in *.scala; do grep -q '^// jars:' "$f" || echo "${f%.scala}"; done | head -3 | tr '\n' '|')
err=$(ls tests/errors/*.scala | head -2 | sed 's|.*/||; s|\.scala$||; s|^|errors-|' | tr '\n' '|')
export IDENTITY_PROGRAMS="${plain}${err%|}"
identity() { COURSIER_CACHE=$cache JOBS=4 timeout 300 tests/support/identity.sh "$@" > "$work/identity.out" 2>&1; }
identity --produce "$work/teq-standin" "$work/bundle"
check "identity --produce" "$?" 0
check "identity --produce: a whole bundle" "$(tail -1 "$work/bundle/identity.txt")" complete
check "identity --produce: bound to the commit" "$(sed -n 's/^commit //p' "$work/bundle/identity.txt")" "$(git rev-parse HEAD)"
check "identity --produce: the jar named by its digest" "$(grep -c '^jar jars/[0-9a-f]\{16\}-scala-library-3.8.4.jar ' "$work/bundle/identity.txt")" 1
identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp1"
check "identity --compare of the same compiler" "$?" 0
has "identity --compare: every program identical" "$work/identity.out" "identity js: 3 of 3 programs identical"
STANDIN_DIFF=$(cut -d'|' -f1 <<< "$plain") identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp2"
check "identity --compare of a differing program" "$?" 1
has "identity --compare: the program named" "$work/identity.out" "differs js $(cut -d'|' -f1 <<< "$plain")"
STANDIN_VERSION="teq 9.9.9 def pgo" identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp3"
check "identity --compare of another version" "$?" 1
has "identity --compare: refused before a run" "$work/identity.out" "prints 'teq 9.9.9 def pgo'"
STANDIN_KILL=$(cut -d'|' -f2 <<< "$plain") identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp4"
check "identity --compare of a run that died" "$?" 1
cp -a "$work/bundle" "$work/bundle-bad"
sed -i 's/^commit .*/commit 0000000000000000000000000000000000000000/' "$work/bundle-bad/identity.txt"
identity --compare "$work/bundle-bad" "$work/teq-standin" "$work/cmp5"
check "identity --compare of another commit's bundle" "$?" 1
rm -rf -- "$work/bundle-bad" && cp -a "$work/bundle" "$work/bundle-bad"
echo tampered >> "$work/bundle-bad/$(sed -n 's/^jar \([^ ]*\) .*/\1/p' "$work/bundle-bad/identity.txt" | head -1)"
identity --compare "$work/bundle-bad" "$work/teq-standin" "$work/cmp6"
check "identity --compare with a jar not the bundle's" "$?" 1
rm -rf -- "$work/bundle-bad" && cp -a "$work/bundle" "$work/bundle-bad"
sed -i '$d' "$work/bundle-bad/identity.txt"
identity --compare "$work/bundle-bad" "$work/teq-standin" "$work/cmp7"
check "identity --compare of a bundle not whole" "$?" 1
rm -rf -- "$work/bundle-bad" && cp -a "$work/bundle" "$work/bundle-bad"
sed -i '1d' "$work/bundle-bad/inventory.txt"
identity --compare "$work/bundle-bad" "$work/teq-standin" "$work/cmp8"
check "identity --compare of another inventory" "$?" 1
STANDIN_KILL=$(cut -d'|' -f1 <<< "$plain") identity --produce "$work/teq-standin" "$work/bundle-killed"
check "identity --produce with a run that died" "$?" 1
STANDIN_VERSION="teq 9.9.9 0000000 pgo" identity --produce "$work/teq-standin" "$work/bundle-other"
check "identity --produce with a binary of another commit" "$?" 1
has "identity --produce: says so" "$work/identity.out" "not built from this checkout's commit"
STANDIN_KILL_WORKER=$(cut -d'|' -f2 <<< "$plain") identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp10"
check "identity --compare with a worker lost" "$?" 1
has "identity --compare: the program left without a result" "$work/identity.out" "no result"
withjars=$(cd tests/cases && grep -l '^// jars: ' ./*.scala | head -1 | sed 's|^\./||; s|\.scala$||')
IDENTITY_PROGRAMS=$withjars identity --produce "$work/teq-standin" "$work/bundle-jars"
check "identity --produce without a program's jars" "$?" 1
has "identity --produce: the missing jars named" "$work/identity.out" "missing js $withjars:"
echo "js $withjars" > "$work/skips"
IDENTITY_PROGRAMS=$withjars IDENTITY_SKIPS=$work/skips identity --produce "$work/teq-standin" "$work/bundle-skips" "js"
check "identity --produce with an allowed skip" "$?" 0
IDENTITY_PROGRAMS=$withjars identity --compare "$work/bundle-skips" "$work/teq-standin" "$work/cmp9"
check "identity --compare of a bundle with a skip" "$?" 0
has "identity --compare: the skip counted" "$work/identity.out" "0 of 0 programs identical to the reference's, 1 skipped"
# One identity run at a time in the checkout: one while another holds out/identity.lock refused, the lock of a run
# gone taken over (a lock the suite made, never another run's).
if mkdir out/identity.lock 2> /dev/null; then
  echo $$ > out/identity.lock/pid
  identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp11"
  check "identity while another run holds the lock: refused, the lock kept" "$?/$(cat out/identity.lock/pid)/$([ -e "$work/cmp11" ] && echo written)" "1/$$/"
  has "identity: names the other run" "$work/identity.out" "another identity run uses out/identity in this checkout, pid $$"
  sh -c 'exit 0' &
  gone=$!
  wait "$gone"
  echo "$gone" > out/identity.lock/pid
  identity --compare "$work/bundle" "$work/teq-standin" "$work/cmp12"
  check "identity with the lock of a run gone: taken over, released" "$?/$([ -e out/identity.lock ] && echo held)" "0/"
  has "identity: says the lock was taken over" "$work/identity.out" "whose run is gone, taken over"
else
  echo "skip the identity lock: another identity run holds out/identity.lock"
fi
unset IDENTITY_PROGRAMS

# --- The admission, against a GitHub of a bare repository, a stand-in gh, and a Central of the landed mirror
# (bench/release-mirror.py) serving sbt-teq 1.0.0.
gh=$work/github.git co=$work/admit
git_ init -q --bare "$gh"
mkdir -p "$co/bench/actions" "$co/integrations/sbt" "$work/central/releases" "$work/central/maven/build/teq/sbt-teq_sbt2_3/1.0.0"
for f in pom jar; do echo "sbt-teq 1.0.0" > "$work/central/maven/build/teq/sbt-teq_sbt2_3/1.0.0/sbt-teq_sbt2_3-1.0.0.$f"; done
python3 -B bench/release-mirror.py "$work/central.port" "$work/central/releases" "$work/central/maven" 2> /dev/null &
pids="$pids $!"
for _ in $(seq 1 50); do [ -s "$work/central.port" ] && break; sleep 0.1; done
central_root=http://127.0.0.1:$(cat "$work/central.port")/maven
cp bench/ship-release.sh bench/ship-qualified.txt "$co/bench/"
cp bench/actions/admit.sh bench/actions/ship.Dockerfile bench/actions/toolchain.sh bench/actions/image.sh "$co/bench/actions/"
recipe=$(bench/actions/image.sh recipe)
image=ghcr.io/carrot-inc/teq-ship@sha256:$(printf '%064d' 1)
# version <v> [<plugin>]: the two files naming the compiler's version, and the plugin it selects (1.0.0 by default).
version() {
  printf '[package]\nname = "teq"\nversion = "%s"\n' "$1" > "$co/Cargo.toml"
  printf '[[package]]\nname = "teq"\nversion = "%s"\n' "$1" > "$co/Cargo.lock"
  echo "${2:-1.0.0}" > "$co/integrations/sbt/plugin-version.txt"
}
# images <reference and recipe of the amd64 image, or none> <the qualified file's image>: ship-image.txt's two
# lines, the arm64 image's of the same recipe.
image_arm=ghcr.io/carrot-inc/teq-ship@sha256:$(printf '%064d' 9)
images() {
  if [ "$1" = none ]; then printf '# the images\nnone\n'; else printf '# the images\n%s linux/amd64\n%s %s linux/arm64\n' "$1" "$image_arm" "${1#* }"; fi > "$co/bench/actions/ship-image.txt"
  grep -v '^image ' bench/ship-qualified.txt > "$co/bench/ship-qualified.txt"
  [ -z "$2" ] || echo "image $2" >> "$co/bench/ship-qualified.txt"
}
commit() { (cd "$co" && git_ add -A && git_ commit -q -m "$1" && git_ push -q origin HEAD:refs/heads/master && git rev-parse HEAD); }
version 0.1.6
images "$image $recipe" "$image"
(cd "$co" && git_ init -q && git_ remote add origin "$gh") || exit 1
a=$(commit "Base")
version 0.1.7
b=$(commit "Release 0.1.7")
echo x > "$co/other"
c=$(commit "Land something")
mkdir -p "$work/stub-gh"
cat > "$work/bin/gh" << 'EOF'
#!/bin/bash
# The stand-in for gh: `gh api <path> [--jq ...]` answers $STUB_GH/<path's file> when there is one (the --jq
# filter's answer; a file `HTTP 502` a GitHub that does not answer), else 404.
# `gh release download <tag> --repo <repo> [--pattern <name>] --dir <dir>` copies $STUB_GH/release-<tag>/ (the one
# file), `gh release view <tag> ... --json assets` lists it (`release not found` without it; a file `HTTP 502` there
# a GitHub that does not answer), and `gh run download <run> --repo <repo> --name <artifact> --dir <dir>`
# $STUB_GH/run-<run>-<artifact>/.
if [ "$1 $2" = "release download" ] || [ "$1 $2" = "release view" ]; then
  verb=$2 tag=$3 dir= pattern=
  shift 3
  while [ $# -gt 0 ]; do case $1 in --dir) dir=$2; shift ;; --pattern) pattern=$2; shift ;; esac; shift; done
  [ ! -f "$STUB_GH/release-$tag/HTTP 502" ] || { echo "HTTP 502: Bad Gateway" >&2; exit 1; }
  [ -d "$STUB_GH/release-$tag" ] || { echo "release not found" >&2; exit 1; }
  if [ "$verb" = view ]; then ls "$STUB_GH/release-$tag"; exit 0; fi
  cp "$STUB_GH/release-$tag"/${pattern:-*} "$dir/"; exit
fi
[ "$1 $2" != "run download" ] || { [ -d "$STUB_GH/run-$3-$7" ] && cp -R "$STUB_GH/run-$3-$7/." "$9/"; exit; }
[ "$1" = api ] || { [ "$1 $2" = "run list" ] && { cat "$STUB_GH/runs" 2> /dev/null; exit 0; }; exit 2; }
key=$(printf '%s' "$2" | tr '/?&=' '____')
# A listing of runs is empty, and a run's artifacts none, but where a file says otherwise.
case $key in *_runs_status_*) [ -f "$STUB_GH/$key" ] || exit 0 ;; *_artifacts_*) [ -f "$STUB_GH/$key" ] || { echo 0; exit 0; } ;; esac
[ -f "$STUB_GH/$key" ] || { echo "HTTP 404: Not Found" >&2; exit 1; }
[ "$(cat "$STUB_GH/$key")" != "HTTP 502" ] || { echo "HTTP 502: Bad Gateway" >&2; exit 1; }
cat "$STUB_GH/$key"
EOF
chmod +x "$work/bin/gh"
# admit <event> <sha> [<var>=<value>...]: the admission in a clone of GitHub's repository; its outputs in $work/admit.outputs.
admit() {
  local event=$1 sha=$2
  shift 2
  rm -rf -- "$work/admit-run" && git clone -q "$gh" "$work/admit-run" && : > "$work/admit.outputs" || exit 1
  (cd "$work/admit-run" && env PATH="$work/bin:$PATH" STUB_GH="$work/stub-gh" GITHUB_REPOSITORY=Carrot-Inc/teq GITHUB_REF=refs/heads/master \
    GITHUB_EVENT_NAME="$event" GITHUB_SHA="$sha" GITHUB_RUN_ID=10 GITHUB_EVENT_PATH="$work/event.json" GITHUB_OUTPUT="$work/admit.outputs" \
    TEQ_CENTRAL_ROOT="$central_root" TEQ_VERSION= TEQ_DRY= TEQ_RESUME_RUN= "$@" timeout 120 bench/actions/admit.sh) > "$work/admit.out" 2>&1
}
out() { sed -n "s/^$1=//p" "$work/admit.outputs"; }
echo '{}' > "$work/event.json"
(cd "$co" && git_ push -q origin "$b:refs/heads/master" -f)
admit push "$b"
check "admit: a push of Release 0.1.7" "$?/$(out go)/$(out version)/$(out commit)/$(out mode)/$(out qualified)/$(out plugin)/$(out plugin_publish)" "0/true/0.1.7/$b/fresh/true/1.0.0/false"
check "admit: the arm64 image, not qualified by the file" "$(out image)/$(out image_arm64)/$(out qualified_arm64)" "$image/$image_arm/false"
check "admit: the profiles, the previous release's" "$(out train)/$(out profiles)" "false/v0.1.6"
(cd "$co" && git_ push -q origin "$c:refs/heads/master" -f)
printf '{"before": "%s"}' "$b" > "$work/event.json"
admit push "$c"
check "admit: a push with no release" "$?/$(out go)" "0/false"
printf '{"before": "%s"}' "$a" > "$work/event.json"
admit push "$c"
check "admit: a push holding a release below its head" "$?" 1
has "admit: says to dispatch it" "$work/admit.out" "is not its release commit"
admit push "$b" GITHUB_REPOSITORY=someone/teq
check "admit: a fork's push" "$?" 1
admit push "$b" GITHUB_REF=refs/heads/feature
check "admit: another branch" "$?" 1
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7
check "admit: a dispatch of 0.1.7 releases its commit" "$?/$(out go)/$(out commit)" "0/true/$b"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_TRAIN=true TEQ_PROFILES=v0.1.5
check "admit: a dispatch asking for a training, another release's profiles named" "$?/$(out train)/$(out profiles)" "0/true/v0.1.5"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_PROFILES='v0.1.5 x'
check "admit: profiles of no tag" "$?" 1
admit workflow_dispatch "$c" TEQ_VERSION=0.1.8
check "admit: a dispatch of a version master has no commit of" "$?" 1
admit workflow_dispatch "$c" TEQ_VERSION=1.0
check "admit: a dispatch of no version" "$?" 1
admit workflow_dispatch "$c" TEQ_DRY=true
check "admit: a dry run on master's version" "$?/$(out go)/$(out version)/$(out commit)/$(out dry)" "0/true/0.1.7/$c/true"
admit workflow_dispatch "$c" TEQ_DRY=true TEQ_VERSION=0.1.6
check "admit: a dry run of another version than master's" "$?" 1
admit workflow_dispatch "$c" TEQ_DRY=true TEQ_RESUME_RUN=5
check "admit: a dry run resuming" "$?" 1
echo 5 > "$work/stub-gh/repos_Carrot-Inc_teq_actions_workflows_release.yml_runs_status_in_progress_per_page_100"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7
check "admit: an earlier run in progress" "$?" 1
has "admit: names it" "$work/admit.out" "release runs 5 are not done"
echo 20 > "$work/stub-gh/repos_Carrot-Inc_teq_actions_workflows_release.yml_runs_status_in_progress_per_page_100"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7
check "admit: a later run in progress, which refuses itself" "$?/$(out go)" "0/true"
rm -f -- "$work/stub-gh/repos_Carrot-Inc_teq_actions_workflows_release.yml_runs_status_in_progress_per_page_100"
(cd "$co" && git_ tag v0.1.7 "$b" && git_ push -q origin v0.1.7)
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7
check "admit: the tag exists, its release not published" "$?" 1
echo false > "$work/stub-gh/repos_Carrot-Inc_teq_releases_tags_v0.1.7"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7
check "admit: the release done already" "$?/$(out go)" "0/false"
has "admit: says so" "$work/admit.out" "released on"
echo ".github/workflows/release.yml Carrot-Inc/teq master workflow_dispatch completed" > "$work/stub-gh/repos_Carrot-Inc_teq_actions_runs_111"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=111
check "admit: a resume of the run 111" "$?/$(out go)/$(out mode)/$(out source_run)" "0/true/resume/111"
check "admit: a resume of a run that left no record decides the plugin as a new run" "$(out plugin_publish)" false
# A resume publishes the plugin as the run it resumes began to, by that run's record (its artifact ship-record),
# though Central serves the plugin that run promoted (1.0.0 here) since.
echo 1 > "$work/stub-gh/repos_Carrot-Inc_teq_actions_runs_111_artifacts_per_page_100"
resumed() {
  python3 -B bench/ship-record.py "$work/stub-gh/run-111-ship-record/0.1.7/record.json" set compiler.version 0.1.7 compiler.commit "$b" plugin.version 1.0.0 "$@" > /dev/null
}
resumed plugin.publish yes
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=111
check "admit: a resume of a run that publishes the plugin, served since: published, as the record says" "$?/$(out plugin)/$(out plugin_publish)" "0/1.0.0/true"
resumed_publish=$(out plugin_publish)
resumed plugin.publish no
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=111
check "admit: a resume of a run that publishes no plugin" "$?/$(out plugin_publish)" "0/false"
resumed plugin.publish yes compiler.commit "$c"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=111
check "admit: a resume whose record is another commit's: refused" "$?" 1
has "admit: names the record's commit" "$work/admit.out" "is of 0.1.7 on ${c:0:12}"
rm -rf -- "$work/stub-gh/run-111-ship-record"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=111
check "admit: a resume whose record cannot be downloaded: refused" "$?" 1
echo "HTTP 502" > "$work/stub-gh/repos_Carrot-Inc_teq_actions_runs_111_artifacts_per_page_100"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=111
check "admit: a resume whose run's artifacts cannot be read: refused" "$?" 1
rm -f -- "$work/stub-gh/repos_Carrot-Inc_teq_actions_runs_111_artifacts_per_page_100"
echo ".github/workflows/gate.yml Carrot-Inc/teq master push completed" > "$work/stub-gh/repos_Carrot-Inc_teq_actions_runs_112"
admit workflow_dispatch "$c" TEQ_VERSION=0.1.7 TEQ_RESUME_RUN=112
check "admit: a resume of another workflow's run" "$?" 1
version 0.1.8
images none ""
d=$(commit "Release 0.1.8")
admit workflow_dispatch "$d" TEQ_VERSION=0.1.8
check "admit: no image" "$?" 1
version 0.1.9
images "$image $(printf '%064d' 2)" "$image"
e=$(commit "Release 0.1.9")
admit workflow_dispatch "$e" TEQ_VERSION=0.1.9
check "admit: an image of another recipe" "$?" 1
images "$image $recipe" ""
sed -i '/linux\/arm64$/d' "$co/bench/actions/ship-image.txt"
admit workflow_dispatch "$(commit "Name one image")" TEQ_DRY=true
check "admit: no arm64 image" "$?" 1
has "admit: names the platform" "$work/admit.out" "names no linux/arm64 image"
version 0.1.10
images "$image $recipe" ""
f=$(commit "Release 0.1.10")
admit workflow_dispatch "$f" TEQ_VERSION=0.1.10
check "admit: an image the qualified file does not name" "$?" 1
has "admit: says a dry run qualifies it" "$work/admit.out" "a dry run qualifies it"
admit workflow_dispatch "$f" TEQ_DRY=true
check "admit: a dry run with an image not qualified" "$?/$(out qualified)" "0/false"
# A history before the plugin's release longer than a pipe holds, as a real master's is.
(cd "$co" && {
  for i in $(seq 1 2500); do
    printf 'commit refs/heads/master\ncommitter t <t@example.invalid> 1700000000 +0000\ndata <<E\nLand the change %s of a long history\nE\n%s\n' "$i" "$([ "$i" = 1 ] && echo "from $(git rev-parse HEAD)")"
  done
} | timeout 120 git fast-import --quiet && git_ push -q origin HEAD:refs/heads/master) || exit 1
version 0.1.10 1.0.1
images "$image $recipe" "$image"
commit "Release sbt-teq 1.0.1" > /dev/null
version 0.1.11 1.0.1
g=$(commit "Release 0.1.11")
admit workflow_dispatch "$g" TEQ_VERSION=0.1.11
check "admit: a release selecting a plugin Central does not serve, its release stated, publishes it" "$?/$(out plugin)/$(out plugin_publish)" "0/1.0.1/true"
admit workflow_dispatch "$g" TEQ_DRY=true
check "admit: a dry run selecting a plugin Central does not serve" "$?" 1
version 0.1.14 1.0.2
g2=$(commit "Release 0.1.14")
admit workflow_dispatch "$g2" TEQ_VERSION=0.1.14
check "admit: a plugin Central does not serve, its release not stated" "$?" 1
has "admit: says so" "$work/admit.out" "no commit 'Release sbt-teq 1.0.2'"
version 0.1.15 0.1.6
g3=$(commit "Release 0.1.15")
admit workflow_dispatch "$g3" TEQ_VERSION=0.1.15
check "admit: a plugin up to 0.1.6" "$?" 1
version 0.1.12
commit "Set 0.1.12" > /dev/null
echo y > "$co/other"
i=$(commit "Release 0.1.12")
admit workflow_dispatch "$i" TEQ_VERSION=0.1.12
check "admit: a release commit that bumped nothing" "$?" 1
version 0.1.13
printf '[[package]]\nname = "teq"\nversion = "0.1.12"\n' > "$co/Cargo.lock"
j=$(commit "Release 0.1.13")
admit workflow_dispatch "$j" TEQ_VERSION=0.1.13
check "admit: files that disagree" "$?" 1

# --- The profiles a release takes up (bench/ship-profiles.sh: an asset written from a ship's tree, whose records of
# before their lines are completed from the tree, the same bytes for the same profiles; its refusals; take into a
# carry, the source named), and the profiles job's decision (bench/actions/profiles.sh, the stand-in gh serving the
# release's assets, bench/pgo.sh ship stale stood in for): a training asked for, a release or an asset not there, an
# asset not whole, another compiler's, stale; else taken, its carry for the builds.
pt=$work/ptree
mkdir -p "$pt/target/pgo/ship" "$pt/target/cross" "$pt/target/ship"
printf '[package]\nname = "teq"\nversion = "0.1.6"\n' > "$pt/Cargo.toml"
printf '[[package]]\nname = "teq"\nversion = "0.1.6"\n' > "$pt/Cargo.lock"
(cd "$pt" && git_ init -q && git_ add -A && git_ commit -q -m "Release 0.1.6") || exit 1
ptc=$(git -C "$pt" rev-parse HEAD)
rustc_now=$(rustc -vV | sed -n 's/^release: //p') rustc_hash=$(rustc -vV | sed -n 's/^commit-hash: //p')
echo 'c  a.scala' > "$pt/target/pgo/ship/corpus.txt" && echo 'j  a.jar' > "$pt/target/pgo/ship/jars.txt"
# The 0.1.6 ship's records, before the lines of the release workflow's: no metadata, release, machine or toolchain.
for p_ in target/pgo/ship/teq target/cross/arm; do
  echo "counts of $p_" > "$pt/$p_.profdata"
  printf 'training status ok\ntraining programs 65\ntraining runs 72\ntraining corpus %s 1 files\ntraining jars %s 1 jars\ntraining profile %s\n' \
    "$(sha256sum < "$pt/target/pgo/ship/corpus.txt" | cut -c1-64)" "$(sha256sum < "$pt/target/pgo/ship/jars.txt" | cut -c1-64)" "$(sha256sum < "$pt/$p_.profdata" | cut -c1-64)" > "$pt/$p_.training"
done
mv "$pt/target/pgo/ship/teq.training" "$pt/target/pgo/ship/training.txt"
printf 'training give-ways 1\ntraining trainer aarch64-unknown-linux-musl under qemu-user 10.2.1 q\n' >> "$pt/target/cross/arm.training"
printf 'binary b\nhost x86_64-unknown-linux-gnu\ntuple rustc %s %s\ntuple llvm 22.1.8\ntuple os Ubuntu 26.04 resolute\n' "$rustc_now" "$rustc_hash" > "$pt/target/ship/teq.manifest"
profiles() { timeout 60 bench/ship-profiles.sh "$@" > "$work/profiles.out" 2>&1; }
profiles write "$pt" "$work/assets-0.1.6"
check "ship-profiles write without the metadata: refused" "$?" 1
echo 0123abcd > "$pt/target/cross/metadata"
profiles write "$pt" "$work/assets-0.1.6"
check "ship-profiles write of the 0.1.6 ship's tree" "$?/$(ls "$work/assets-0.1.6")" "0/teq-0.1.6-profiles.tar"
asset=$work/assets-0.1.6/teq-0.1.6-profiles.tar
check "ship-profiles: the records completed from the tree" \
  "$(tar -xOf "$asset" target/cross/arm.training | grep -E '^training (metadata|release|host|trainer|tuple (rustc|qemu-user)) ' | tr '\n' '|')" \
  "training trainer aarch64-unknown-linux-musl under qemu-user 10.2.1 q|training metadata 0123abcd|training release 0.1.6 $ptc|training host x86_64-unknown-linux-gnu|training tuple rustc $rustc_now $rustc_hash|training tuple qemu-user 10.2.1 q|"
check "ship-profiles: the x86-64 trainer native" "$(tar -xOf "$asset" target/pgo/ship/training.txt | sed -n 's/^training trainer //p')" "x86_64-unknown-linux-gnu native"
profiles write "$pt" "$work/assets-again"
check "ship-profiles: the same bytes again" "$(cmp "$asset" "$work/assets-again/teq-0.1.6-profiles.tar" && echo same)" same
check "ship-profiles check" "$(profiles check "$asset"; echo $?)" 0
mkdir -p "$work/bad" && tar -xf "$asset" -C "$work/bad" && echo more >> "$work/bad/target/cross/arm.profdata"
(cd "$work/bad" && tar -cf "$work/bad.tar" profiles.txt target)
check "ship-profiles check of a profile not its record's" "$(profiles check "$work/bad.tar"; echo $?)" 1
(cd "$work/bad" && tar -cf "$work/bad.tar" profiles.txt)
check "ship-profiles check of an asset without its files" "$(profiles check "$work/bad.tar"; echo $?)" 1
profiles take "$asset" v0.1.6 "$work/taken"
check "ship-profiles take: the products in the carry, each record naming its source" \
  "$?/$(cd "$work/taken" && find . -type f | LC_ALL=C sort | tr '\n' ' ')/$(sed -n 's/^training source //p' "$work/taken/target/cross/arm.training")" \
  "0/./target/cross/arm.profdata ./target/cross/arm.training ./target/pgo/ship/corpus.txt ./target/pgo/ship/jars.txt ./target/pgo/ship/teq.profdata ./target/pgo/ship/training.txt /v0.1.6 $(sha256sum < "$asset" | cut -c1-64)"
# The profiles job, in a tree of the scripts whose bench/pgo.sh stands in for the stale build (STUB_STALE its exit).
pj=$work/pjob
mkdir -p "$pj/bench/actions"
cp bench/actions/profiles.sh "$pj/bench/actions/" && cp bench/ship-profiles.sh bench/ship-manifest.sh bench/ship-release.sh "$pj/bench/"
printf '#!/bin/bash\ncd "$(dirname "$0")/.." && mkdir -p target/pgo/ship && echo "stale 3 of the 200 hottest functions changed, 10 allowed" > target/pgo/ship/stale.txt\nexit ${STUB_STALE:-0}\n' > "$pj/bench/pgo.sh"
chmod +x "$pj/bench/pgo.sh"
# The stand-in gh alone before the real tools: the image's rustc names the compiler.
mkdir -p "$work/bin-gh" && ln -sf "$work/bin/gh" "$work/bin-gh/gh"
decide() {
  rm -f "$work/pjob.outputs" "$work/pjob.tar"
  (cd "$pj" && env PATH="$work/bin-gh:$PATH" STUB_GH="$work/stub-gh" GITHUB_OUTPUT="$work/pjob.outputs" TEQ_TRAIN= TEQ_PROFILES=v0.1.6 "$@" \
    timeout 120 bench/actions/profiles.sh "$work/pjob-carry" "$work/pjob.tar") > "$work/pjob.out" 2>&1
  echo "$?/$(sed -n 's/^train=//p' "$work/pjob.outputs" 2> /dev/null)"
}
check "profiles job: a training asked for" "$(decide TEQ_TRAIN=true)" 0/true
check "profiles job: no release of the tag" "$(decide)" 0/true
has "profiles job: says so" "$work/pjob.out" "GitHub has no release v0.1.6"
mkdir -p "$work/stub-gh/release-v0.1.6" && echo b > "$work/stub-gh/release-v0.1.6/teq-0.1.6-linux-x86_64"
check "profiles job: a release without the asset" "$(decide)" 0/true
cp "$work/bad.tar" "$work/stub-gh/release-v0.1.6/teq-0.1.6-profiles.tar"
check "profiles job: an asset not whole" "$(decide)" 0/true
has "profiles job: says why" "$work/pjob.out" "is not whole"
sed "s/^training tuple rustc .*/training tuple rustc 1.97.0 old/" "$pt/target/pgo/ship/training.txt" > "$work/training.old" && cp "$pt/target/pgo/ship/training.txt" "$work/training.kept"
grep -v '^training tuple' "$work/training.old" > "$pt/target/pgo/ship/training.txt" && echo "training tuple rustc 1.97.0 old" >> "$pt/target/pgo/ship/training.txt"
profiles write "$pt" "$work/assets-old" && cp "$work/assets-old/teq-0.1.6-profiles.tar" "$work/stub-gh/release-v0.1.6/"
cp "$work/training.kept" "$pt/target/pgo/ship/training.txt"
check "profiles job: another compiler's profiles" "$(decide)" 0/true
has "profiles job: names the compiler" "$work/pjob.out" "trained by rustc 1.97.0 old"
cp "$asset" "$work/stub-gh/release-v0.1.6/"
check "profiles job: profiles stale for the tree" "$(decide STUB_STALE=3)" 0/true
has "profiles job: says how stale" "$work/pjob.out" "are stale for this tree"
check "profiles job: profiles taken" "$(decide)/$(sed -n 's/^source=//p' "$work/pjob.outputs")" "0/false/v0.1.6 $(sha256sum < "$asset" | cut -c1-64)"
check "profiles job: their carry for the builds" "$(tar -tf "$work/pjob.tar" | grep -c '\.profdata$')/$(tar -xOf "$work/pjob.tar" target/pgo/ship/training.txt | grep -c '^training source v0.1.6 ')" "2/1"
check "profiles job: a stale build that failed" "$(decide STUB_STALE=1)" "1/"
echo > "$work/stub-gh/release-v0.1.6/HTTP 502"
check "profiles job: GitHub not answering" "$(decide)" "1/"
rm -rf -- "$work/stub-gh/release-v0.1.6"

# --- The publication: bench/ship-publish.sh's steps through bench/actions/release-step.sh, in a repository of the landed
# scripts whose GitHub release, smoke, plugin publisher and Portal client are stand-ins that log their calls, the
# Portal a directory of the deployment it holds, which refuses a second upload of the version as the companion's
# guard does; GitHub's downloads and Central the landed mirror. The record and the Central record saved and put back
# on "another runner" where a job is lost.
q=$work/publication
mkdir -p "$q/bench/actions" "$q/integrations/sbt/example" "$work/portal"
cp bench/ship-publish.sh bench/ship-record.py bench/ship-release.sh bench/ship-manifest.sh "$q/bench/"
cp bench/actions/release-step.sh "$q/bench/actions/"
printf '[package]\nname = "teq"\nversion = "0.1.7"\n' > "$q/Cargo.toml"
printf '[[package]]\nname = "teq"\nversion = "0.1.7"\n' > "$q/Cargo.lock"
echo 1.0.0 > "$q/integrations/sbt/plugin-version.txt"
echo "teq: 0.1.6" > "$q/integrations/sbt/example/teq.lock"
printf 'out/\nintegrations/sbt/binary/binaries/\n' > "$q/.gitignore"
cat > "$q/bench/github-release.sh" << 'EOF'
#!/bin/bash
echo "github-release $*" >> "$STUB_LOG"
[ "${2:-}" != "${STUB_GITHUB_FAIL:-none}" ] || exit 3
EOF
cat > "$q/bench/release-smoke.sh" << 'EOF'
#!/bin/bash
echo "smoke $*" | sed "s|$PWD/||g" >> "$STUB_LOG"
exit "${STUB_SMOKE_EXIT:-0}"
EOF
cat > "$q/bench/release.sh" << 'EOF'
#!/bin/bash
[ "$*" = "--pin --commit" ] || exit 2
echo "teq: 0.1.7" > integrations/sbt/example/teq.lock
git -c commit.gpgsign=false commit -q -am "Pin the example and the documents to 0.1.7"
EOF
cat > "$q/integrations/sbt/publish.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/../.." || exit 1
echo "publish.sh $*" >> "$STUB_LOG"
r=out/central/sbt-teq-1.0.0
case ${1:-} in
  --preflight) exit 0 ;;
  # The staging recorded as central.py check records it, at the head.
  --stage) mkdir -p "$r/staging/build/teq/sbt-teq_sbt2_3/1.0.0" && echo pom > "$r/staging/build/teq/sbt-teq_sbt2_3/1.0.0/sbt-teq_sbt2_3-1.0.0.pom" &&
    echo bundle > "$r/bundle.zip" && echo "{\"event\": \"staged\", \"head\": \"$(git rev-parse HEAD)\"}" >> "$r/deployment.log" ;;
  "")
    # The publish resumes the recorded deployment and reads it back; without one it would upload afresh.
    grep -q '"event": "upload-started"' "$r/deployment.log" 2> /dev/null || { echo "publish.sh: no deployment recorded"; exit 1; }
    exit "${STUB_READBACK_EXIT:-0}" ;;
esac
EOF
cat > "$q/integrations/sbt/central.py" << 'EOF'
import json, os, sys
open(os.environ["STUB_LOG"], "a").write("central.py %s\n" % " ".join(a.replace(os.getcwd() + "/", "") for a in sys.argv[1:]))
portal = os.path.join(os.environ["STUB_PORTAL_DIR"], "deployment")
command, record = sys.argv[1], sys.argv[2]
log = os.path.join(record, "deployment.log")
def event(name):
    open(log, "a").write(json.dumps({"event": name}) + "\n")
def events():
    return [json.loads(line) for line in open(log)] if os.path.exists(log) else []
uploaded = any(e["event"] == "upload-started" for e in events())
if command == "state":
    print(os.environ.get("STUB_PORTAL") or (open(portal).read().strip() if uploaded and os.path.exists(portal) else "none"))
elif command == "staged":
    print(next((e["head"] for e in reversed(events()) if e["event"] == "staged"), ""))
elif command == "upload":
    if uploaded:
        sys.exit("central.py: the record has an upload already")
    if os.path.exists(portal):
        sys.exit("central.py: another deployment names the version on the Portal")
    open(portal, "w").write("validated\n")
    event("upload-started")
elif command == "wait":
    sys.exit(int(os.environ.get("STUB_WAIT_EXIT", "0")))
elif command == "promote":
    event("promote-requested")
    open(portal, "w").write("published\n")
EOF
chmod +x "$q"/bench/*.sh "$q"/integrations/sbt/publish.sh
cp "$root/NOTICE" "$root/LICENSE" "$q/"
(cd "$q" && git_ init -q && git_ add -A && git_ commit -q -m "Release 0.1.7") || exit 1
qr=$(cd "$q" && git rev-parse HEAD)
# The five staged as bench/ship.sh stages them, built from that commit.
qs=$work/publication-staged
mkdir -p "$qs/profiles" && echo "the profiles of 0.1.7" > "$qs/profiles/teq-0.1.7-profiles.tar"
for cl in osx-aarch_64 osx-x86_64 linux-x86_64 linux-aarch_64 windows-x86_64; do
  mkdir -p "$qs/binaries/$cl" && echo "teq 0.1.7 of $cl" > "$qs/binaries/$cl/teq" &&
    printf 'binary %s\nversion teq 0.1.7 %s pgo\ncommit %s\n' "$(sha256sum < "$qs/binaries/$cl/teq" | cut -c1-64)" "${qr:0:9}" "$qr" > "$qs/binaries/$cl/teq.manifest"
done
# The five qualifications' reports, as bench/actions/qualify.sh writes them on each build's products.
qq=$work/qualified
qualified_reports() {
  local cl sha
  rm -rf -- "$qq" && mkdir -p "$qq" || exit 1
  for cl in osx-aarch_64 osx-x86_64 linux-x86_64 linux-aarch_64 windows-x86_64; do
    sha=$(sha256sum < "$qs/binaries/$cl/teq" | cut -c1-64)
    { echo "qualify: $cl: on Linux"; echo "qualify: $cl: the binary $sha, 'teq 0.1.7 ${qr:0:9} pgo', runs"
      case $cl in osx-* | linux-aarch_64) echo "qualify: $cl: its outputs are the Linux x86-64 reference's, the binary $(sha256sum < "$qs/binaries/linux-x86_64/teq" | cut -c1-64) (0123456789abcdef)" ;; esac
      echo "qualify: $cl: qualified"; } > "$qq/$cl.txt"
  done
}
qualified_reports
mkdir -p "$work/github/releases" "$work/github/maven/build/teq/sbt-teq_sbt2_3/1.0.0"
for f in pom jar; do echo "sbt-teq 1.0.0" > "$work/github/maven/build/teq/sbt-teq_sbt2_3/1.0.0/sbt-teq_sbt2_3-1.0.0.$f"; done
python3 -B bench/release-mirror.py "$work/github.port" "$work/github/releases" "$work/github/maven" 2> /dev/null &
pids="$pids $!"
for _ in $(seq 1 50); do [ -s "$work/github.port" ] && break; sleep 0.1; done
qport=$(cat "$work/github.port")
# qenv: the run's variables and the stand-ins'; qstep <var>=<value>... -- <verb> [<args>]: the driver in that repository
# under them, its calls in $work/q.log.
qenv=(PATH="$work/bin:$PATH" STUB_GH="$work/stub-gh" STUB_LOG="$work/q.log" STUB_PORTAL_DIR="$work/portal"
  TEQ_RELEASES_BASE="http://127.0.0.1:$qport/releases/download" TEQ_CENTRAL_ROOT="http://127.0.0.1:$qport/maven"
  TEQ_VERSION=0.1.7 TEQ_COMMIT="$qr" TEQ_RUN=10 TEQ_DRY=false TEQ_MODE=fresh TEQ_STAGED="$qs" TEQ_QUALIFIED="$qq" TEQ_PLUGIN=1.0.0 TEQ_PLUGIN_PUBLISH=true
  TEQ_CENTRAL_CREDENTIALS=x GITHUB_RUN_ID=10 TEQ_RETRY_PAUSE=0 GH_TOKEN=made-up)
qstep() {
  local -a vars=()
  while [ "$1" != -- ]; do vars+=("$1"); shift; done
  shift
  (cd "$q" && env "${qenv[@]}" "${vars[@]}" timeout 300 bench/actions/release-step.sh "$@") > "$work/q.out" 2>&1
}
qfresh() { rm -rf -- "$q/out" "$work/portal" && mkdir -p "$work/portal" && : > "$work/q.log"; }
qget() { python3 -B bench/ship-record.py "$q/out/ship/0.1.7/record.json" get "$1"; }
qset() { python3 -B bench/ship-record.py "$q/out/ship/0.1.7/record.json" set "$@"; }
qcount() { grep -c -- "$1" "$work/q.log"; }
# The persisted records (the workflow's artifacts), saved and put back on another runner.
qsave() { rm -rf -- "$work/saved-$1" && mkdir -p "$work/saved-$1" && cp -a "$q/out/ship" "$work/saved-$1/ship" && { [ ! -d "$q/out/central" ] || cp -a "$q/out/central" "$work/saved-$1/central"; }; }
qlose() {
  rm -rf -- "$q/out" && mkdir -p "$q/out" && cp -a "$work/saved-$1/ship" "$q/out/ship" &&
    { [ ! -d "$work/saved-$2/central" ] || cp -a "$work/saved-$2/central" "$q/out/central"; }
}
qall() { for s_ in draft stage,smoke upload-begin upload promote read-back publish smoke-public; do qstep -- publish "$s_" || return 1; done; }
qfresh
(cd "$q" && rm -rf -- integrations/sbt/binary/binaries && mkdir -p integrations/sbt/binary/binaries && cp -R "$qs/binaries/." integrations/sbt/binary/binaries/ &&
  env "${qenv[@]}" timeout 300 bench/ship-publish.sh 0.1.7 --plugin 1.0.0) > "$work/q.out" 2>&1
check "bench/ship-publish.sh without --step: the whole publication in one run" \
  "$?/$(qget github.state)/$(qget central.state)/$(qget smoke.public)/$(qcount 'central.py upload')/$(qcount 'central.py promote')" "0/read-back/read-back/passed/1/1"
has "bench/ship-publish.sh without --step: says the release is out" "$work/q.out" "released 0.1.7"
qfresh
qall
check "publication: every step of a plugin's release, a job each" \
  "$?/$(qget github.state)/$(qget central.state)/$(qget smoke.mirror)/$(qget smoke.public)/$(qget workflow.run)" "0/read-back/read-back/passed/passed/10"
check "publication: one preflight, one stage, one upload, one promotion, one read-back, one publication" \
  "$(qcount 'publish.sh --preflight')/$(qcount 'publish.sh --stage')/$(qcount 'central.py upload')/$(qcount 'central.py promote')/$(grep -cx 'publish.sh ' "$work/q.log")/$(qcount 'github-release 0.1.7 publish')" "1/1/1/1/1/1"
has "publication: the smoke against the staged set and the staging" "$work/q.log" "smoke --mirror out/github-release out/central/sbt-teq-1.0.0/staging 0.1.7"
qfresh
for s_ in draft smoke publish smoke-public; do qstep TEQ_PLUGIN_PUBLISH=false -- publish "$s_" || break; done
check "publication: a compiler's release alone, the plugin Central serves" "$?/$(qget github.state)/$(qget central.state)/$(qcount 'publish.sh')" "0/read-back/served/0"
qfresh
qstep -- publish draft
qstep -- publish promote
check "publication: the promotion before the upload refused" "$?/$(qcount 'central.py promote')" "1/0"
qstep -- publish publish
check "publication: the publication before the plugin's read-back refused" "$?/$(qcount 'github-release 0.1.7 publish')" "1/0"
qstep -- smoke
check "publication: the smoke of a plugin's release before its staging refused" "$?/$(qcount 'smoke --mirror')" "1/0"
# A read-back that failed after the promotion, the job run again: the deployment resumed, no fresh preflight.
qfresh
for s_ in draft stage,smoke upload-begin upload promote; do qstep -- publish "$s_" || break; done
qstep STUB_READBACK_EXIT=1 -- publish read-back
check "publication: a read-back failing after the promotion keeps the draft" "$?/$(qget github.state)/$(qcount abandon)" "1/draft/0"
qstep -- publish read-back && qstep -- publish publish
check "publication: the read-back run again, then the publication: no second preflight or upload" \
  "$?/$(qget central.state)/$(qcount 'publish.sh --preflight')/$(qcount 'central.py upload')" "0/read-back/1/1"
# A runner lost during the upload: the staging and the intent persisted, the deployment's record not.
qfresh
qstep -- publish draft && qstep -- publish stage,smoke && qsave staged && qstep -- publish upload-begin && qsave intent && qstep -- publish upload
qlose intent staged
qstep -- publish upload
check "publication: the upload after a runner lost during the upload, refused by the Portal" "$?/$(cat "$work/portal/deployment")/$(qget github.state)" "1/validated/deleted"
has "publication: says to reconcile on the Portal" "$work/q.out" "reconciled on the Portal"
# A runner lost after the upload, its records persisted: the promotion on another runner, no fresh preflight.
qfresh
qstep -- publish draft && qstep -- publish stage,smoke && qstep -- publish upload-begin && qstep -- publish upload && qsave uploaded
qlose uploaded uploaded
qstep -- publish promote && qstep -- publish read-back
check "publication: promoted on another runner after the upload" "$?/$(qget central.state)/$(qcount 'publish.sh --preflight')" "0/read-back/1"
# A staging that is not the head's: not uploaded.
qfresh
qstep -- publish draft && qstep -- publish stage,smoke && qstep -- publish upload-begin
sed -i 's/"head": "[0-9a-f]*"/"head": "0000000"/' "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep -- publish upload
check "publication: the upload of a staging not the head's refused, the draft abandoned" "$?/$(qcount 'central.py upload')/$(qget github.state)" "1/0/deleted"
# The resume's checks.
cp "$qs/binaries/linux-x86_64/teq" "$work/teq.saved" && echo changed > "$qs/binaries/linux-x86_64/teq"
qstep -- publish publish
check "publication: a resume with other binaries than the record's refused" "$?" 1
cp "$work/teq.saved" "$qs/binaries/linux-x86_64/teq"
qfresh
qstep -- publish smoke
check "publication: a step after the draft with no record refused" "$?" 1
qstep -- publish draft && qset github.state deleted > /dev/null
qstep -- publish smoke
check "publication: a step after an abandoned draft refused" "$?" 1
qstep -- assets "$work/assets"
check "assets: the release's files by the landed names" "$?/$(cd "$work/assets" && ls | LC_ALL=C sort | tr '\n' ' ')" \
  "0/LICENSE NOTICE SHA256SUMS teq-0.1.7-binaries.txt teq-0.1.7-linux-aarch_64 teq-0.1.7-linux-x86_64 teq-0.1.7-osx-aarch_64 teq-0.1.7-osx-x86_64 teq-0.1.7-profiles.tar teq-0.1.7-windows-x86_64.exe "
check "assets: the manifest's header" "$(head -1 "$work/assets/teq-0.1.7-binaries.txt")" "teq 0.1.7 $qr"
mkdir -p "$work/stub-gh/release-v0.1.7" && cp "$work/assets"/* "$work/stub-gh/release-v0.1.7/"
qfresh
qstep -- draft
check "draft: the draft made, its assets the staged set's" "$?/$(qget github.state)" "0/draft"
has "draft: the staged five the qualified ones" "$work/q.out" "the staged five are the binaries their qualifications ran"
sed -i "s/the binary [0-9a-f]*, /the binary $(printf '%064d' 5), /" "$qq/osx-x86_64.txt"
qfresh
qstep -- draft
check "draft whose staged binary its qualification did not run: refused, nothing drafted" "$?/$(qcount 'github-release 0.1.7')" "1/0"
has "draft: names the binary" "$work/q.out" "the staged osx-x86_64 is not the binary its qualification ran"
qualified_reports
sed -i "s/reference's, the binary [0-9a-f]* /reference's, the binary $(printf '%064d' 6) /" "$qq/linux-aarch_64.txt"
qfresh
qstep -- draft
check "draft whose outputs were compared with another reference: refused" "$?" 1
qualified_reports
sed -i '$d' "$qq/windows-x86_64.txt"
qfresh
qstep -- draft
check "draft with a qualification not passed: refused" "$?" 1
qualified_reports
echo other > "$work/stub-gh/release-v0.1.7/teq-0.1.7-linux-x86_64"
qfresh
qstep -- draft
check "draft whose asset is not the staged set's: refused" "$?" 1
has "draft: names the asset" "$work/q.out" "the draft's teq-0.1.7-linux-x86_64 is not the staged set's"
cp "$work/assets"/* "$work/stub-gh/release-v0.1.7/"
# A dispatch resume after the first run promoted the plugin, which Central serves since: the new run's draft job,
# with the admission's decision from the first run's record (above), takes the publication up with the plugin; the
# decision made again (no plugin, Central serving it) is refused by the record.
qfresh
for s_ in draft stage,smoke upload-begin upload promote; do qstep -- publish "$s_" || break; done
qstep STUB_READBACK_EXIT=1 -- publish read-back
qsave first && qlose first first
qstep TEQ_MODE=resume GITHUB_RUN_ID=12 TEQ_PLUGIN_PUBLISH=false -- draft
check "resume with the plugin decided again: refused, the draft kept" "$?/$(qget github.state)" "1/draft"
has "resume with the plugin decided again: says to resume as it began" "$work/q.out" "resume it as it began"
qstep TEQ_MODE=resume GITHUB_RUN_ID=12 TEQ_PLUGIN_PUBLISH="$resumed_publish" -- draft &&
  qstep GITHUB_RUN_ID=12 TEQ_PLUGIN_PUBLISH="$resumed_publish" -- publish read-back &&
  qstep GITHUB_RUN_ID=12 TEQ_PLUGIN_PUBLISH="$resumed_publish" -- publish publish
check "resume with the admission's decision from the record: read back and published, nothing uploaded or promoted again" \
  "$?/$(qget central.state)/$(qget github.state)/$(qcount 'central.py upload')/$(qcount 'central.py promote')/$(qcount 'publish.sh --preflight')" "0/read-back/read-back/1/1/1"

# --- The conclusion, by the record and GitHub's and the Portal's answers.
qrecord() { qfresh && qstep -- publish draft > /dev/null && : > "$work/q.log" && qset "$@" > /dev/null; }
rm -f -- "$work/stub-gh/repos_Carrot-Inc_teq_releases_tags_v0.1.7"
for word in PUBLISHED publishing read-back something; do
  qrecord central.state publishing && mkdir -p "$q/out/central/sbt-teq-1.0.0" && echo '{"event": "upload-started"}' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
  qstep STUB_PORTAL=$word -- conclude
  check "conclude, the promotion begun and the Portal saying $word: the draft kept" "$?/$(qcount abandon)" "0/0"
done
qrecord central.state uploaded && mkdir -p "$q/out/central/sbt-teq-1.0.0" && echo '{"event": "upload-started"}' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep STUB_PORTAL=VALIDATED -- conclude
check "conclude, the deployment validated, not promoted: abandoned and dropped" "$?/$(qcount 'abandon')/$(qcount 'central.py drop')/$(qget github.state)" "0/1/1/deleted"
# A runner lost after the promotion, before its records were persisted: the conclusion has the records from before
# it (central.state uploaded, no promote-requested), asks the Portal, which says PUBLISHED, and keeps the draft.
qfresh
qstep -- publish draft && qstep -- publish stage,smoke && qstep -- publish upload-begin && qstep -- publish upload && qsave uploaded
qstep -- publish promote
check "the promotion made on the runner then lost" "$?/$(cat "$work/portal/deployment")" "0/published"
qlose uploaded uploaded
: > "$work/q.log"
qstep -- conclude
check "conclude after a promotion whose runner was lost: the Portal asked, the draft kept" \
  "$?/$(qcount abandon)/$(qcount 'central.py state')/$(qcount 'central.py drop')/$(qget github.state)/$(qget central.state)" "0/0/1/0/draft/uploaded"
has "conclude after a promotion whose runner was lost: says the plugin is on Central" "$work/q.out" "is on Central"
qrecord central.state uploaded && mkdir -p "$q/out/central/sbt-teq-1.0.0" && echo '{"event": "upload-started"}' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep STUB_PORTAL=unanswered -- conclude
check "conclude after the upload, the Portal not telling: the draft kept" "$?/$(qcount abandon)" "0/0"
has "conclude: says to ask the Portal" "$work/q.out" "does not say whether it was promoted"
qrecord central.state uploaded && mkdir -p "$q/out/central/sbt-teq-1.0.0" && echo '{"event": "upload-started"}' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep TEQ_CENTRAL_CREDENTIALS= -- conclude
check "conclude after the upload, the Portal not to be asked: the draft kept" "$?/$(qcount abandon)" "0/0"
echo "HTTP 502" > "$work/stub-gh/repos_Carrot-Inc_teq_releases_tags_v0.1.7"
qrecord central.state staged
qstep -- conclude
check "conclude with GitHub silent: nothing deleted" "$?/$(qcount abandon)/$(qget github.state)" "0/0/draft"
has "conclude with GitHub silent: says so" "$work/q.out" "GitHub did not answer"
rm -f -- "$work/stub-gh/repos_Carrot-Inc_teq_releases_tags_v0.1.7"
qrecord central.state staged && mkdir -p "$q/out/central/sbt-teq-1.0.0" && echo '{"event": "staged"}' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep STUB_PORTAL=PUBLISHED -- conclude
check "conclude before an upload: abandoned, the Portal not asked" "$?/$(qcount abandon)/$(qcount 'central.py state')" "0/1/0"
qrecord central.state uploading && mkdir -p "$q/out/central/sbt-teq-1.0.0" && echo '{"event": "staged"}' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep -- conclude
check "conclude after an upload begun and not recorded: abandoned" "$?/$(qcount abandon)/$(qcount 'central.py drop')" "0/1/0"
has "conclude: says to drop the upload on the Portal" "$work/q.out" "dropped there first"
qrecord central.state publishing && mkdir -p "$q/out/central/sbt-teq-1.0.0" && printf '{"event": "upload-started"}\n{"event": "promote-requested"}\n' > "$q/out/central/sbt-teq-1.0.0/deployment.log"
qstep -- conclude
check "conclude, the promotion requested by the record: the draft kept, the Portal not needed" "$?/$(qcount abandon)" "0/0"
has "conclude: says to resume" "$work/q.out" "resume_run=10"
qrecord github.state deleted
qstep -- conclude
has "conclude after the draft's abandonment: says a new run starts afresh" "$work/q.out" "starts afresh"
echo false > "$work/stub-gh/repos_Carrot-Inc_teq_releases_tags_v0.1.7"
qrecord central.state staged
qstep -- conclude
check "conclude after the publication: nothing abandoned" "$?/$(qcount abandon)" "0/0"
has "conclude: says the assets are immutable" "$work/q.out" "is public"
rm -f -- "$work/stub-gh/repos_Carrot-Inc_teq_releases_tags_v0.1.7"
qfresh
qstep TEQ_DRY=true -- conclude
check "conclude of a dry run: the scratch draft deleted" "$?/$(cat "$work/q.log")" "0/github-release 0.0.0-dry abandon"
: > "$work/q.log"
qstep -- conclude
check "conclude with no record: nothing published" "$?/$(qcount abandon)" "0/0"

# --- The pin, onto a GitHub of a bare repository: bench/release.sh --pin --commit as the Actions bot, pushed without force.
pg=$work/pin.git
git_ init -q --bare "$pg"
(cd "$q" && git_ remote add origin "$pg" && git_ push -q origin HEAD:refs/heads/master) || exit 1
qpin() { qfresh && mkdir -p "$q/out/ship/0.1.7" && qset github.state read-back smoke.public passed > /dev/null && (cd "$q" && git_ checkout -q --detach "$qr"); qstep -- pin; }
qpin
check "pin onto the release's master, by the Actions bot" "$?/$(git --git-dir="$pg" log -1 --format='%s/%an/%cn' master)/$(qget pin.state)" \
  "0/Pin the example and the documents to 0.1.7/github-actions[bot]/github-actions[bot]/pushed"
qpin
check "pin again: on master already, nothing pushed" "$?/$(git --git-dir="$pg" rev-list --count master)" "0/2"
pin_again() {
  (cd "$q" && git_ push -q -f origin "$qr:refs/heads/master") || exit 1
  (cd "$work" && rm -rf -- other && git clone -q "$pg" other && cd other && eval "$1" && git_ add -A && git_ commit -q -m "$2" && git_ push -q origin HEAD:refs/heads/master) || exit 1
}
pin_again 'echo more > README.md' "Land the README"
qpin
check "pin onto a master moved beside it" "$?/$(git --git-dir="$pg" log --format=%s master | tr '\n' '|')" "0/Pin the example and the documents to 0.1.7|Land the README|Release 0.1.7|"
pin_again 'echo "teq: 0.1.5" > integrations/sbt/example/teq.lock' "Land the example"
qpin
check "pin onto a master that changed the example: refused" "$?/$(git --git-dir="$pg" log -1 --format=%s master)/$(qget pin.state)" "1/Land the example/refused"
pin_again 'echo 1.0.1 > integrations/sbt/plugin-version.txt' "Release sbt-teq 1.0.1"
qpin
check "pin onto a master that releases again: refused" "$?/$(git --git-dir="$pg" log -1 --format=%s master)" "1/Release sbt-teq 1.0.1"
qfresh && mkdir -p "$q/out/ship/0.1.7" && qset github.state draft > /dev/null
qstep -- pin
check "pin before the publication: refused" "$?" 1

# --- The landing's guard.
hub=$work/hub.git ghub=$work/ghub.git l=$work/land
git_ init -q --bare "$hub" && git_ init -q --bare "$ghub"
(git_ init -q "$l" && cd "$l" && git_ remote add origin "$hub" && git_ remote add github "$ghub" && echo a > a && git_ add a && git_ commit -q -m base &&
  git_ push -q origin HEAD:refs/heads/master && git_ push -q github HEAD:refs/heads/master) || exit 1
land() { (cd "$l" && PATH="$work/bin:$PATH" STUB_GH="$work/stub-gh" timeout 120 "$root/bench/actions/before-landing.sh") > "$work/land.out" 2>&1; }
: > "$work/stub-gh/runs"
land
check "before-landing: the two masters the same" "$?" 0
echo "77 in_progress" > "$work/stub-gh/runs"
land
check "before-landing while a release runs" "$?" 1
: > "$work/stub-gh/runs"
(cd "$l" && echo b > a && git_ commit -q -am "Pin the example and the documents to 0.1.7" && git_ push -q github HEAD:refs/heads/master && git_ reset -q --hard HEAD~1) || exit 1
land
check "before-landing: GitHub ahead by the pin, the hub fast-forwarded" "$?/$(git --git-dir="$hub" log -1 --format=%s master)" "0/Pin the example and the documents to 0.1.7"
(cd "$l" && git_ fetch -q github && git_ checkout -q github/master && echo c > a && git_ commit -q -am "Something on GitHub" && git_ push -q github HEAD:refs/heads/master) || exit 1
land
check "before-landing: GitHub ahead by another commit" "$?" 1
(cd "$l" && git_ checkout -q origin/master && echo d > d && git_ add d && git_ commit -q -m "A landing" && git_ push -q origin HEAD:refs/heads/master) || exit 1
land
check "before-landing: the two diverged" "$?" 1

# --- The built jars restored under another checkout's prefix, newer than their sources.
mkdir -p "$work/jars-in/scratch-dir" && echo j > "$work/jars-in/scratch-x.jar" && echo k > "$work/jars-in/scratch-dir/y.jar"
tar -czf "$work/jars.tgz" -C "$work/jars-in" scratch-x.jar scratch-dir
prefix=$(TMPDIR=$work/tmp bash -c '. tests/support/jars.sh && echo "$scratch"')
mkdir -p "$work/tmp"
TMPDIR=$work/tmp timeout 60 bench/actions/jars.sh restore "$work/jars.tgz" > /dev/null
check "jars restore" "$(cat "$prefix-x.jar" "$prefix-dir/y.jar" 2> /dev/null | tr '\n' ' ')" "j k "
check "jars restore: newer than the sources" "$([ "$prefix-x.jar" -nt tests/support/jars.sh ] && echo newer)" newer

# --- The credentials, with a key made for this run and a tree whose signing key it is.
k=$work/keys
mkdir -p "$k/gnupg" "$k/tree/bench/actions" && chmod 700 "$k/gnupg"
if GNUPGHOME=$k/gnupg timeout 60 gpg --batch --quiet --pinentry-mode loopback --passphrase '' --quick-gen-key "teq test <t@example.invalid>" ed25519 sign never 2> /dev/null; then
  fpr=$(GNUPGHOME=$k/gnupg gpg --batch --with-colons --list-secret-keys 2> /dev/null | awk -F: '$1 == "fpr" { print $10; exit }')
  key=$(GNUPGHOME=$k/gnupg gpg --batch --armor --export-secret-keys "$fpr" 2> /dev/null)
  cp bench/actions/credentials.sh "$k/tree/bench/actions/"
  echo "release_signing_key=$fpr" > "$k/tree/bench/ship-release.sh"
  creds() { (cd "$k/tree" && env "$@" timeout 120 bench/actions/credentials.sh write "$k/out") > "$k/creds.out" 2> "$k/creds.err"; }
  creds CENTRAL_USER=user1 CENTRAL_PASSWORD='p4ss=word!' GPG_PRIVATE_KEY="$key"
  check "credentials write" "$?/$(cat "$k/creds.out" | tr '\n' ' ')" "0/TEQ_CENTRAL_CREDENTIALS=$k/out/central GNUPGHOME=$k/out/gnupg "
  check "credentials: the properties" "$(cat "$k/out/central" 2> /dev/null | tr '\n' ' ')" "host=central.sonatype.com user=user1 password=p4ss=word! "
  check "credentials: the files are the runner user's alone" "$(stat -c %a "$k/out/central" "$k/out/gnupg" | tr '\n' ' ')" "600 700 "
  lacks "credentials: the password printed nowhere" "$k/creds.out" "p4ss"
  lacks "credentials: the password printed nowhere" "$k/creds.err" "p4ss"
  creds CENTRAL_USER=user1 CENTRAL_PASSWORD=x GPG_PRIVATE_KEY="$key"
  check "credentials write over a directory there" "$?" 1
  (cd "$k/tree" && timeout 60 bench/actions/credentials.sh remove "$k/out") > /dev/null 2>&1
  check "credentials remove" "$?/$([ -e "$k/out" ] && echo left)" "0/"
  creds CENTRAL_USER=user1 CENTRAL_PASSWORD='back\slash' GPG_PRIVATE_KEY="$key"
  check "credentials: a backslash refused" "$?" 1
  rm -rf -- "$k/out"
  creds CENTRAL_USER=user1 CENTRAL_PASSWORD=x
  check "credentials without the key" "$?" 1
  echo "release_signing_key=$(printf '%040d' 7)" > "$k/tree/bench/ship-release.sh"
  rm -rf -- "$k/out"
  creds CENTRAL_USER=user1 CENTRAL_PASSWORD=x GPG_PRIVATE_KEY="$key"
  check "credentials with another key than the release's" "$?" 1
  rm -rf -- "$k/out"
  GNUPGHOME=$k/gnupg timeout 30 gpgconf --kill all 2> /dev/null
else
  echo "skip bench/actions/credentials.sh: gpg makes no key here"
fi

# --- The image's preflight outside the image, and its recipe.
check "toolchain check outside the image" "$(bench/actions/toolchain.sh check > /dev/null 2>&1; echo $?)" 1
check "image recipe" "$(bench/actions/image.sh recipe)" "$(cat bench/actions/ship.Dockerfile bench/actions/toolchain.sh | sha256sum | cut -c1-64)"

echo "release-actions: passed: $pass of $((pass + fail))"
[ $fail = 0 ]
