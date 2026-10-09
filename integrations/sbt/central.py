#!/usr/bin/env python3
"""central.py <command> [<options>]: a release of sbt-teq's way to Maven Central through the Central Portal's
publisher API, for publish.sh, which runs each step. The compiler's binaries
never go to Central (the GitHub release is theirs): the plugin alone does. sbt stages and signs the plugin
(`publishSigned` into `localStaging`); this script checks what is staged, bundles it, uploads the bundle as a
USER_MANAGED deployment, which the Portal validates and nothing publishes by itself, and then promotes that
deployment by its id, or drops it, and reads the published plugin back from the public repository. Central never
takes a published file back, so the promotion is a step of its own, refused unless the deployment is the one
recorded here, validated, its bundle unchanged since the upload and its version a release's (never a
rehearsal's).

The record is a directory per release of the plugin, `out/central/sbt-teq-<version>` at the checkout's root (a
rehearsal's `out/central/sbt-teq-<version>-<run>`): `staging/` the Maven tree sbt staged, `manifest.tsv` every staged file with its
size and digests, `bundle.zip`, and `deployment.log`, one JSON event a line, appended and synced before and
after each request to the Portal, so that a run cut off (a client timeout, the ship's bound, the machine) is
resumed from the deployment it recorded and never uploads the release a second time.

Commands:
  preflight [--version <v>]         the token file (~/.sbt/sonatype_central_credentials, Java properties: host
                                    central.sonatype.com, user, password; TEQ_CENTRAL_CREDENTIALS another) and
                                    the Portal taking it for the namespace build.teq; with a version, that the
                                    Portal does not publish the plugin at it and holds no live deployment of it
  keyserver <fingerprint>           the public key on keyserver.ubuntu.com, which the Portal reads signatures by
  check <record> <version> <fingerprint> <head>
                                    the staged tree is the plugin's files exactly (its pom, jar, sources and
                                    javadoc jars), each with its .asc (a signature by the key), .md5 and .sha1
                                    (its digests), the pom with what Central requires, the jar built from the
                                    head at the version; writes manifest.tsv and bundle.zip
  upload <record> <mode>            uploads bundle.zip, USER_MANAGED; mode `release` or `rehearsal`
  wait <record> <state> <seconds>   polls the deployment until it is VALIDATED or PUBLISHED, within the bound
  promote <record> <head>           publishes the recorded deployment: a release's, validated, its bundle and
                                    staging the manifest's, recorded at the head
  drop <record>                     drops the recorded deployment (VALIDATED or FAILED)
  state <record>                    prints the deployment's state: `none` without an upload, `absent` when the
                                    upload made none, `unanswered` when an upload without an answer is not found
                                    among the namespace's deployments (yet), `dropped`, `read-back`, or the Portal's
  staged <record>                   prints the head the record's staging was made at, nothing without one
  abandon <record>                  gives up an unanswered upload the Portal still does not list, the operator's
                                    decision after looking at the Portal: the record then reads `absent`
  readback <record> <root> <seconds>
                                    reads every staged file back from the public repository under <root>
                                    (https://repo1.maven.org/maven2/), without credentials, until each gives the
                                    manifest's bytes (a .sha1 or .md5 its digest), within the bound, each request
                                    bounded by 360 s; what it read is recorded, so a later run reads the rest

One publish of a version runs at a time on a machine (publish.sh holds a lock under the user's configuration
directory), and across machines and records the Portal is asked: before the upload and again before the promotion,
any other deployment named for the version refuses it, whatever its state (a rerun on a fresh runner without the
record finds its earlier upload there). An upload's answer is bound to the upload it answers, by its name and its
bundle. Every request to the Portal is bounded whole, its answer read within the bound; progress goes to stderr, a
command's answer (`state`, `staged`) alone to stdout. TEQ_CENTRAL_PORTAL names a stand-in of the Portal for a check of
the scripts, on a loopback address alone, so that the token never goes anywhere else.

The token is read from its file here alone and sent in the Authorization header alone: it is never printed,
logged, put on a command line or into an exception's message. Every request is bounded.
"""
import base64
import datetime
import fcntl
import threading
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
import xml.etree.ElementTree as ET
import zipfile

PORTAL = os.environ.get("TEQ_CENTRAL_PORTAL") or "https://central.sonatype.com/api/v1/publisher"
HOST = "central.sonatype.com"
NAMESPACE = "build.teq"
GROUP_PATH = "build/teq"
MODULE = "sbt-teq_sbt2_3"
RELEASE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
REHEARSAL = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+-rehearsal$")
LIVE = ("PENDING", "VALIDATING", "VALIDATED", "PUBLISHING")
REQUEST = 120
UPLOAD = 1200
READ = 360


class Refused(Exception):
    pass


def say(message):
    print(f"central: {message}", file=sys.stderr, flush=True)


def now():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# The token, from its file: Java properties, as sbt's credentials file reads them.
def credentials():
    path = os.environ.get("TEQ_CENTRAL_CREDENTIALS") or os.path.expanduser("~/.sbt/sonatype_central_credentials")
    if not os.path.isfile(path):
        raise Refused(f"no {path}, the Central Portal's user token (host, user, password)")
    if os.stat(path).st_mode & 0o077:
        raise Refused(f"{path} is readable by others than its owner: chmod 600 it")
    fields = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith(("#", "!")) and "=" in line:
                key, value = line.split("=", 1)
                fields[key.strip()] = value.strip()
    if fields.get("host") != HOST:
        raise Refused(f"{path} names the host {fields.get('host') or 'none'}, not {HOST}")
    missing = [k for k in ("user", "password") if not fields.get(k)]
    if missing:
        raise Refused(f"{path} has no {' and no '.join(missing)}")
    return "Bearer " + base64.b64encode(f"{fields['user']}:{fields['password']}".encode()).decode()


class Portal:
    def __init__(self):
        stand_in = urllib.parse.urlparse(PORTAL)
        if PORTAL != "https://central.sonatype.com/api/v1/publisher" and stand_in.hostname not in ("127.0.0.1", "localhost", "::1"):
            raise Refused(f"TEQ_CENTRAL_PORTAL names {PORTAL}, which is no loopback stand-in of the Portal")
        self.authorization = credentials()

    def request(self, method, path, query=None, body=None, content_type=None, timeout=REQUEST):
        """The answer's status and text, the whole exchange within `timeout`: a socket timeout bounds a silence
        alone, so the request runs in a thread of its own, given up at the bound (its answer, if one comes, is
        never read)."""
        url = f"{PORTAL}/{path}" + (f"?{urllib.parse.urlencode(query)}" if query else "")
        request = urllib.request.Request(url, data=body, method=method)
        request.add_header("Authorization", self.authorization)
        if content_type:
            request.add_header("Content-Type", content_type)
        outcome = {}

        def exchange():
            try:
                with urllib.request.urlopen(request, timeout=timeout) as response:
                    outcome["answer"] = (response.status, response.read().decode("utf-8", "replace"))
            except urllib.error.HTTPError as e:
                outcome["answer"] = (e.code, e.read().decode("utf-8", "replace")[:2000])
            except (urllib.error.URLError, OSError, ValueError) as e:
                outcome["failure"] = f"{method} {url}: no answer ({getattr(e, 'reason', e)})"

        worker = threading.Thread(target=exchange, daemon=True)
        worker.start()
        worker.join(timeout)
        if worker.is_alive():
            raise Refused(f"{method} {url}: no whole answer within {timeout} s")
        if "failure" in outcome:
            raise Refused(outcome["failure"])
        return outcome["answer"]

    def status(self, deployment, timeout=REQUEST):
        code, text = self.request("POST", "status", {"id": deployment}, timeout=timeout)
        if code != 200:
            raise Refused(f"the deployment {deployment}'s status answered {code}: {text}")
        return json.loads(text)

    def deployments(self, name):
        code, text = self.request("GET", "deployments", {"namespace": NAMESPACE, "deploymentName": name, "size": 50})
        if code != 200:
            raise Refused(f"the namespace {NAMESPACE}'s deployments answered {code}: {text}")
        return json.loads(text).get("deployments", [])


def named(portal, version, ours=None):
    """The Portal's deployments named for the plugin's version (`build.teq sbt-teq <version> <run>`), but `ours`."""
    prefix = f"{NAMESPACE} sbt-teq {version} "
    return [d for d in portal.deployments(prefix) if (d.get("deploymentName") or "").startswith(prefix) and d.get("deploymentId") != ours]


def alone(portal, version, ours=None):
    """Refuses while another deployment names the version: two uploads of a release, from two records, two
    checkouts or two runners, are never promoted, the second never uploaded."""
    others = named(portal, version, ours)
    if others:
        raise Refused(f"the Portal holds other deployments of sbt-teq {version}: "
                      + ", ".join(f"{d.get('deploymentId')} '{d.get('deploymentName')}' {d.get('deploymentState')}" for d in others)
                      + "; a release is uploaded once: resume the recorded one, or drop the others (central.py drop) once they are known")


# The record: the release's directory, its events.
def log_path(record):
    return os.path.join(record, "deployment.log")


def events(record):
    if not os.path.isfile(log_path(record)):
        return []
    with open(log_path(record), encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def append(record, event, **fields):
    entry = {"event": event, "at": now(), **fields}
    with open(log_path(record), "a", encoding="utf-8") as f:
        f.write(json.dumps(entry, sort_keys=True) + "\n")
        f.flush()
        os.fsync(f.fileno())
    return entry


def last(record, *names):
    for e in reversed(events(record)):
        if e["event"] in names:
            return e
    return None


def answer_of(record):
    """The recorded answer to the record's upload, the last begun: its `uploaded` event, by the upload's name."""
    started = last(record, "upload-started")
    if not started:
        return None
    return next((e for e in reversed(events(record)) if e["event"] == "uploaded" and e.get("name") == started["name"]), None)


def deployment_of(record):
    answer = answer_of(record)
    if answer:
        return answer["id"]
    if last(record, "upload-started"):
        raise Refused(f"{record}: an upload began without an answer recorded; `central.py state {record}` looks for it by its name")
    raise Refused(f"{record} records no deployment")


def digests(path):
    sha1, md5, sha256, size = hashlib.sha1(), hashlib.md5(), hashlib.sha256(), 0
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            sha1.update(chunk), md5.update(chunk), sha256.update(chunk)
            size += len(chunk)
    return size, sha1.hexdigest(), md5.hexdigest(), sha256.hexdigest()


def manifest(record):
    rows = {}
    with open(os.path.join(record, "manifest.tsv"), encoding="utf-8") as f:
        for line in f:
            path, size, sha1, md5, sha256 = line.rstrip("\n").split("\t")
            rows[path] = (int(size), sha1, md5, sha256)
    return rows


def sha256_of(path):
    return digests(path)[3]


# preflight
def preflight(version=None):
    portal = Portal()
    code, text = portal.request("GET", "deployments", {"namespace": NAMESPACE, "size": 1})
    if code != 200:
        raise Refused(f"the Portal refuses the token for the namespace {NAMESPACE} ({code}: {text})")
    say(f"the token in its file reaches the Portal, which lists {NAMESPACE}'s deployments")
    if version:
        code, text = portal.request("GET", "published", {"namespace": NAMESPACE, "name": MODULE, "version": version})
        if code != 200:
            raise Refused(f"whether {NAMESPACE}:{MODULE}:{version} is published cannot be told ({code}: {text})")
        if json.loads(text).get("published") is not False:
            raise Refused(f"the Portal says {NAMESPACE}:{MODULE}:{version} is published ({text.strip()}): a release is never published again")
        live = [d for d in portal.deployments(f"{NAMESPACE} sbt-teq {version} ") if d.get("deploymentState") in LIVE]
        if live:
            raise Refused(f"the Portal holds live deployments of {version}: "
                          + ", ".join(f"{d.get('deploymentId')} {d.get('deploymentState')}" for d in live)
                          + "; resume the one recorded under out/central/, or drop the others")
        say(f"the Portal does not publish sbt-teq {version} and holds no live deployment of it")


def keyserver(fingerprint):
    url = f"https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x{fingerprint}"
    got = subprocess.run(["curl", "-fsS", "--max-time", "60", url], capture_output=True, timeout=90)
    if got.returncode != 0:
        raise Refused(f"keyserver.ubuntu.com does not give the key {fingerprint}: {got.stderr.decode().strip()}")
    shown = subprocess.run(["gpg", "--batch", "--with-colons", "--show-keys"], input=got.stdout, capture_output=True, timeout=60)
    fingerprints = [line.split(":")[9] for line in shown.stdout.decode().splitlines() if line.startswith("fpr:")]
    if fingerprint.upper() not in [f.upper() for f in fingerprints]:
        raise Refused(f"keyserver.ubuntu.com's answer for {fingerprint} holds the keys {fingerprints}, not it")
    say(f"keyserver.ubuntu.com serves the public key {fingerprint}")


# check: what is staged, against the release's files.
def expected(version):
    files = {}
    base = f"{GROUP_PATH}/{MODULE}/{version}/{MODULE}-{version}"
    for p in (f"{base}.pom", f"{base}.jar", f"{base}-sources.jar", f"{base}-javadoc.jar"):
        files[p] = "primary"
        for sidecar in ("asc", "md5", "sha1"):
            files[f"{p}.{sidecar}"] = sidecar
    return files


def signed_by(signature, data):
    verified = subprocess.run(["gpg", "--batch", "--status-fd", "1", "--verify", signature, data],
                              stdin=subprocess.DEVNULL, capture_output=True, timeout=60)
    for line in verified.stdout.decode().splitlines():
        if line.startswith("[GNUPG:] VALIDSIG "):
            return line.split()[-1].upper()
    return None


POM = "{http://maven.apache.org/POM/4.0.0}"


def pom_problems(path, version):
    root = ET.parse(path).getroot()
    text = lambda p: (root.findtext("/".join(POM + part for part in p.split("/"))) or "").strip()
    wanted = {"groupId": NAMESPACE, "artifactId": MODULE, "version": version, "packaging": "jar"}
    problems = [f"{k} is '{text(k)}', not '{v}'" for k, v in wanted.items() if text(k) != v]
    for required in ("name", "description", "url", "licenses/license/name", "developers/developer/name", "scm/url", "scm/connection"):
        if not text(required):
            problems.append(f"no {required}")
    if root.find(f"{POM}repositories") is not None:
        problems.append("a repositories element, which pomIncludeRepository leaves out")
    return problems


def check(record, version, fingerprint, head):
    if not (RELEASE.match(version) or REHEARSAL.match(version)):
        raise Refused(f"{version} is neither a release's version nor a rehearsal's")
    staging = os.path.join(record, "staging")
    staged = sorted(os.path.relpath(os.path.join(d, f), staging).replace(os.sep, "/")
                    for d, _, names in os.walk(staging) for f in names)
    files = expected(version)
    extra, missing = sorted(set(staged) - set(files)), sorted(set(files) - set(staged))
    if extra or missing:
        raise Refused(f"the staging is not the release's files: {'extra ' + ' '.join(extra) if extra else ''}"
                      f"{'; ' if extra and missing else ''}{'missing ' + ' '.join(missing) if missing else ''}")
    rows = {p: digests(os.path.join(staging, p)) for p in staged}
    problems = []
    for p, kind in files.items():
        full = os.path.join(staging, p)
        if kind == "primary":
            continue
        primary = p.rsplit(".", 1)[0]
        if kind == "asc":
            by = signed_by(full, os.path.join(staging, primary))
            if by != fingerprint.upper():
                problems.append(f"{p} is {'a signature by ' + by if by else 'no good signature'}, not by {fingerprint}")
        else:
            with open(full, encoding="ascii", errors="replace") as f:
                stated = f.read().split()[:1]
            want = rows[primary][1] if kind == "sha1" else rows[primary][2]
            if stated != [want]:
                problems.append(f"{p} states {stated[0] if stated else 'nothing'}, the file's {kind} is {want}")
    pom = f"{GROUP_PATH}/{MODULE}/{version}/{MODULE}-{version}.pom"
    problems += [f"{pom}: {why}" for why in pom_problems(os.path.join(staging, pom), version)]
    jar = os.path.join(staging, f"{GROUP_PATH}/{MODULE}/{version}/{MODULE}-{version}.jar")
    with zipfile.ZipFile(jar) as z:
        info = z.read("dev/teq/sbt/BuildInfo$.class")
    if head.encode() not in info or version.encode() not in info:
        problems.append(f"the plugin's jar does not carry the head {head[:12]} and the version {version} (BuildInfo)")
    if problems:
        raise Refused("the staging does not check out:\n  " + "\n  ".join(problems))
    with open(os.path.join(record, "manifest.tsv"), "w", encoding="utf-8") as f:
        for p in staged:
            size, sha1, md5, sha256 = rows[p]
            f.write(f"{p}\t{size}\t{sha1}\t{md5}\t{sha256}\n")
    # The bundle: the staged files alone, in order, with one timestamp, so that the same staging gives the
    # same bytes.
    bundle = os.path.join(record, "bundle.zip")
    with zipfile.ZipFile(bundle + ".new", "w", zipfile.ZIP_DEFLATED) as z:
        for p in staged:
            entry = zipfile.ZipInfo(p, (2010, 1, 1, 0, 0, 0))
            entry.external_attr = 0o644 << 16
            entry.compress_type = zipfile.ZIP_DEFLATED
            with open(os.path.join(staging, p), "rb") as src:
                z.writestr(entry, src.read())
    os.replace(bundle + ".new", bundle)
    append(record, "staged", version=version, head=head, files=len(staged),
           manifest=sha256_of(os.path.join(record, "manifest.tsv")), bundle=sha256_of(bundle))
    say(f"staged sbt-teq {version}: {len(staged)} files, every signature by {fingerprint}; bundle sha256 {sha256_of(bundle)}")


# The bundle and the staging as the record staged them, before an upload and a promotion.
def unchanged(record):
    staged = last(record, "staged")
    if not staged:
        raise Refused(f"{record} records no staging")
    bundle = os.path.join(record, "bundle.zip")
    if sha256_of(os.path.join(record, "manifest.tsv")) != staged["manifest"] or sha256_of(bundle) != staged["bundle"]:
        raise Refused(f"{record}'s manifest or bundle changed since it was staged")
    staging = os.path.join(record, "staging")
    for p, (size, sha1, _, _) in manifest(record).items():
        if digests(os.path.join(staging, p))[1] != sha1:
            raise Refused(f"{record}/staging/{p} changed since it was staged")
    with zipfile.ZipFile(bundle) as z:
        if sorted(z.namelist()) != sorted(manifest(record)):
            raise Refused(f"{bundle} holds other files than the manifest")
    return staged


def upload(record, mode):
    staged = unchanged(record)
    version = staged["version"]
    if mode not in ("release", "rehearsal") or (mode == "release") != bool(RELEASE.match(version)):
        raise Refused(f"{version} is not a {mode}'s version")
    portal = Portal()
    name = f"{NAMESPACE} sbt-teq {version} {uuid.uuid4().hex[:12]}"
    with open(os.path.join(record, "bundle.zip"), "rb") as f:
        bundle = f.read()
    if hashlib.sha256(bundle).hexdigest() != staged["bundle"]:
        raise Refused(f"{record}/bundle.zip changed since it was staged")
    if last(record, "upload-started"):
        raise Refused(f"{record} has an upload already: a release is uploaded once, and resumed from its record")
    alone(portal, version)
    # The record takes one upload, and the first to claim it under the log's lock makes it.
    with open(log_path(record), "a", encoding="utf-8") as log:
        fcntl.flock(log, fcntl.LOCK_EX)
        if last(record, "upload-started"):
            raise Refused(f"{record} has an upload already: a release is uploaded once, and resumed from its record")
        append(record, "upload-started", name=name, mode=mode, bundle=staged["bundle"])
    boundary = uuid.uuid4().hex
    body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"bundle\"; filename=\"bundle.zip\"\r\n"
            f"Content-Type: application/octet-stream\r\n\r\n").encode() + bundle + f"\r\n--{boundary}--\r\n".encode()
    say(f"uploading {len(bundle)} bytes as the deployment '{name}', USER_MANAGED")
    try:
        code, text = portal.request("POST", "upload", {"name": name, "publishingType": "USER_MANAGED"}, body,
                                    f"multipart/form-data; boundary={boundary}", timeout=UPLOAD)
    except Refused as e:
        append(record, "upload-unanswered", why=str(e))
        raise Refused(f"{e}; the deployment, if the Portal made one, is found by its name: `central.py state {record}`")
    if last(record, "upload-started")["name"] != name or last(record, "staged")["bundle"] != staged["bundle"]:
        raise Refused(f"{record} changed during the upload of '{name}': its answer ({code}) is not recorded")
    if code not in (200, 201) or not re.fullmatch(r"[0-9a-fA-F-]{36}", text.strip()):
        append(record, "upload-refused", name=name, code=code, answer=text[:500])
        raise Refused(f"the upload answered {code}: {text}")
    append(record, "uploaded", name=name, bundle=staged["bundle"], id=text.strip())
    say(f"uploaded: the deployment {text.strip()}")


# The deployment an unanswered upload made, by its name.
def recover(record, portal):
    started = last(record, "upload-started")
    found = portal.deployments(started["name"])
    found = [d for d in found if d.get("deploymentName") == started["name"]]
    if len(found) == 1:
        append(record, "uploaded", name=started["name"], bundle=started["bundle"], id=found[0]["deploymentId"], recovered=True)
        say(f"the unanswered upload made the deployment {found[0]['deploymentId']} ({found[0].get('deploymentState')})")
        return found[0]["deploymentId"]
    if not found:
        # Not yet, perhaps: a deployment the Portal is still making is listed later. Only the operator's
        # `abandon` gives the upload up.
        say(f"the Portal lists no deployment named '{started['name']}' (yet)")
        return None
    raise Refused(f"the Portal holds {len(found)} deployments named '{started['name']}'")


def state(record):
    started = last(record, "upload-started")
    if not started:
        print("none")
        return
    if last(record, "dropped"):
        print("dropped")
        return
    # An upload the Portal refused as a request (4xx) made no deployment; one given up by the operator is gone.
    refused = last(record, "upload-refused")
    abandoned = last(record, "upload-abandoned")
    if not answer_of(record) and ((refused and refused.get("name") == started["name"] and 400 <= refused["code"] < 500)
                                  or (abandoned and abandoned.get("name") == started["name"])):
        print("absent")
        return
    portal = Portal()
    if not answer_of(record):
        if recover(record, portal) is None:
            print("unanswered")
            return
    deployment = deployment_of(record)
    current = portal.status(deployment).get("deploymentState")
    if last(record, "read-back"):
        print("read-back")
    else:
        print(current)


def abandon(record):
    started = last(record, "upload-started")
    if not started or answer_of(record):
        raise Refused(f"{record} has no unanswered upload to give up")
    if recover(record, Portal()) is not None:
        raise Refused(f"the Portal lists the upload '{started['name']}' now: it is resumed, not given up")
    append(record, "upload-abandoned", name=started["name"])
    say(f"gave up the upload '{started['name']}', which the Portal does not list")


def wait(record, wanted, seconds):
    deployment = deployment_of(record)
    portal = Portal()
    deadline, pause, seen, why = time.monotonic() + seconds, 5, None, None
    while True:
        # Every request, and every pause between them, within what is left of the bound: none once it is spent.
        left = deadline - time.monotonic()
        if left <= 0:
            raise Refused(f"the deployment {deployment} is {('still ' + seen) if seen else 'not told'} after {seconds} s"
                          + (f" ({why})" if why and not seen else "") + "; run again to wait on")
        try:
            status = portal.status(deployment, max(0.1, min(REQUEST, left)))
        except Refused as e:
            # A request the Portal did not answer, or answered with an error, is asked again within the bound.
            why = str(e)
            say(f"{e}; asking again")
            time.sleep(max(0.0, min(30, deadline - time.monotonic())))
            continue
        current = status.get("deploymentState")
        if current != seen:
            append(record, "state", state=current)
            say(f"the deployment {deployment} is {current}")
            seen = current
        if current == wanted or (wanted == "VALIDATED" and current in ("PUBLISHING", "PUBLISHED")):
            return
        if current == "FAILED":
            raise Refused(f"the Portal refused the deployment {deployment}:\n" + json.dumps(status.get("errors"), indent=1))
        if wanted == "PUBLISHED" and current == "VALIDATED" and not last(record, "promoted", "promote-requested"):
            raise Refused(f"the deployment {deployment} is VALIDATED and not promoted: nothing publishes it but `promote`")
        time.sleep(max(0.0, min(pause, deadline - time.monotonic())))
        pause = min(pause * 2, 30)


def promote(record, head):
    staged = unchanged(record)
    version = staged["version"]
    if not RELEASE.match(version) or last(record, "upload-started").get("mode") != "release":
        raise Refused(f"{version} is not a release's: a rehearsal is never published")
    if staged["head"] != head:
        raise Refused(f"the deployment holds the release of {staged['head'][:12]}, not of the head {head[:12]}")
    answer = answer_of(record)
    if last(record, "upload-started")["bundle"] != staged["bundle"] or not answer or answer.get("bundle") != staged["bundle"]:
        raise Refused("the deployment recorded is not the upload of the staged bundle")
    if last(record, "dropped"):
        raise Refused(f"{record}'s deployment was dropped")
    deployment = deployment_of(record)
    portal = Portal()
    current = portal.status(deployment).get("deploymentState")
    if current in ("PUBLISHING", "PUBLISHED"):
        if not last(record, "promoted"):
            append(record, "promoted", id=deployment, seen=current)
        say(f"the deployment {deployment} is {current} already")
        return
    if current != "VALIDATED":
        raise Refused(f"the deployment {deployment} is {current}, not VALIDATED")
    alone(portal, version, deployment)
    append(record, "promote-requested", id=deployment)
    code, text = portal.request("POST", f"deployment/{deployment}")
    if code != 204:
        raise Refused(f"the promotion of {deployment} answered {code}: {text}; `central.py state {record}` tells whether it took")
    append(record, "promoted", id=deployment)
    say(f"promoted the deployment {deployment}: {NAMESPACE}:sbt-teq {version} goes to Maven Central")


def drop(record):
    if last(record, "dropped"):
        say(f"{record}'s deployment is dropped already")
        return
    if last(record, "promote-requested"):
        raise Refused(f"{record}'s deployment was promoted: Central keeps what it publishes")
    deployment = deployment_of(record)
    code, text = Portal().request("DELETE", f"deployment/{deployment}")
    if code != 204:
        raise Refused(f"dropping {deployment} answered {code}: {text}")
    append(record, "dropped", id=deployment)
    say(f"dropped the deployment {deployment}")


# readback: the public repository, without credentials, by curl, each request bounded.
def fetch(url, into, bound):
    got = subprocess.run(["curl", "-sS", "--max-time", str(bound), "-o", into, "-w", "%{http_code}", url],
                         stdin=subprocess.DEVNULL, capture_output=True, timeout=bound + 30)
    return got.stdout.decode().strip() or "000"


def readback(record, root, seconds):
    root = root if root.endswith("/") else root + "/"
    rows = manifest(record)
    start = last(record, "promoted", "promote-requested")
    if not start:
        raise Refused(f"{record}'s deployment is not promoted: nothing is published to read back")
    done = {e["path"] for e in events(record) if e["event"] == "served" and e.get("root") == root}
    began, pause, rounds = time.monotonic(), 60, 0
    with tempfile.TemporaryDirectory() as tmp:
        while True:
            rounds += 1
            wrong = []
            for path in sorted(set(rows) - done):
                size, sha1, md5, _ = rows[path]
                # Each request within what is left of the bound.
                left = int(began + seconds - time.monotonic())
                if left < 1:
                    break
                code = fetch(root + path, os.path.join(tmp, "file"), min(READ, left))
                if code != "200":
                    continue
                if path.endswith((".sha1", ".md5")):
                    want = rows[path.rsplit(".", 1)[0]][1 if path.endswith(".sha1") else 2]
                    with open(os.path.join(tmp, "file"), encoding="ascii", errors="replace") as f:
                        served = (f.read().split() or ["nothing"])[0].lower()
                    ok = served == want
                else:
                    served = digests(os.path.join(tmp, "file"))[1]
                    ok = served == sha1
                if ok:
                    done.add(path)
                    append(record, "served", path=path, root=root)
                else:
                    wrong.append(f"{path}: {served}, staged {sha1 if not path.endswith(('.sha1', '.md5')) else 'its digest'}")
            elapsed = int(time.monotonic() - began)
            say(f"read back {len(done)} of {len(rows)} files under {root} after {elapsed // 60} min {elapsed % 60} s (round {rounds})")
            if wrong:
                raise Refused("the repository serves other bytes than were staged:\n  " + "\n  ".join(wrong))
            if done == set(rows):
                append(record, "read-back", root=root, files=len(rows))
                say(f"every file of the release is served as staged, {elapsed // 60} min after this read-back began, "
                    f"promoted at {start['at']}")
                return
            if time.monotonic() + pause > began + seconds:
                unserved = sorted(set(rows) - done)
                raise Refused(f"after {elapsed} s, {len(unserved)} files are not served yet (first {unserved[0]}): the "
                              f"publication is incomplete, not failed; the deployment stays recorded, and running the "
                              f"publish again reads the rest back")
            time.sleep(pause)
            pause = min(pause * 2, 300)


def main(argv):
    command, args = argv[1], argv[2:]
    if command == "preflight":
        preflight(args[1] if args[:1] == ["--version"] else None)
    elif command == "keyserver":
        keyserver(*args)
    elif command == "check":
        check(*args)
    elif command == "upload":
        upload(*args)
    elif command == "wait":
        wait(args[0], args[1], int(args[2]))
    elif command == "promote":
        promote(*args)
    elif command == "drop":
        drop(*args)
    elif command == "state":
        state(*args)
    elif command == "abandon":
        abandon(*args)
    elif command == "staged":
        staged = last(args[0], "staged")
        if staged:
            print(staged["head"])
    elif command == "readback":
        readback(args[0], args[1], int(args[2]))
    else:
        raise Refused(f"no command {command}")


if __name__ == "__main__":
    try:
        main(sys.argv)
    except Refused as e:
        print(f"central: {e}", file=sys.stderr, flush=True)
        sys.exit(1)
    except (IndexError, ValueError, TypeError) as e:
        print(f"central: {type(e).__name__}: {e} (the usage is this file's docstring)", file=sys.stderr, flush=True)
        sys.exit(2)
    except Exception as e:
        # No traceback: the message alone, which never holds the token.
        print(f"central: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
        sys.exit(1)
