#!/usr/bin/env python3
"""bench/typed-once/setup.py <work dir> <teq binary>: a build of the tree's own corpora for the project verbs, written
into a fresh <work dir>: bench/app's generated `shared`, `frontend` and `api` (out/budget/app, bench/programs.sh's)
copied under it, one munit suite beside the API, and a teq.lock of two projects: `web` (JavaScript: shared and
frontend, the class catalog declared cacheable state, as bench/programs.sh checks it) and `api` (the JVM: shared and
api, with a test configuration holding the suite). The jars are the coursier cache's (tests/support/jars.sh's keys and
the example build's munit); nothing is fetched. Run from the repository's root. Prints the work directory."""
import hashlib, importlib.util, json, os, shutil, subprocess, sys

work, teq = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
root = os.getcwd()
if os.path.exists(work):
    sys.exit("setup: %s exists; name a fresh directory" % work)
corpus = os.path.join(root, "out/budget/app")
for part in ("shared", "frontend", "api"):
    if not os.path.isdir(os.path.join(corpus, part)):
        sys.exit("setup: no %s/%s (bench/programs.sh makes the corpora)" % (corpus, part))
    shutil.copytree(os.path.join(corpus, part), os.path.join(work, part))
os.makedirs(os.path.join(work, "apitest"))
open(os.path.join(work, "apitest/ApiSuite.scala"), "w").write(
    "package apitest\n\nclass ApiSuite extends munit.FunSuite:\n  test(\"one\") {\n    assertEquals(1 + 1, 2)\n  }\n")

spec = importlib.util.spec_from_file_location("lock", os.path.join(root, "integrations/sbt/example/check-export.py"))
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
example = lock.parse(open(os.path.join(root, "integrations/sbt/example/teq.lock"), encoding="utf-8").read())

def jar_of(name):
    return subprocess.run(["bash", "-c", ". tests/support/jars.sh; jar_of " + name], capture_output=True, text=True, check=True).stdout.strip()
cache_root = None
def artifact(path):
    global cache_root
    data = open(path, "rb").read()
    cache_root = path[: path.index("/maven2/") + len("/maven2/")]
    parts = path[len(cache_root):].split("/")
    return ":".join([".".join(parts[:-3]), parts[-3], parts[-2]]), "maven-central %s %d" % (hashlib.sha1(data).hexdigest(), len(data))
table = {}
jars = []
for name in ("scala-library", "cats-kernel", "cats-core", "sourcecode"):
    key, record = artifact(jar_of(name))
    table[key] = record
    jars.append(key)
lib3 = os.path.join(cache_root, "org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar")
key, record = artifact(lib3)
table[key] = record
jars.append(key)
munit = [e for e in example["projects"]["jvmapp"]["configurations"]["test"]["classpath"] if isinstance(e, str) and e not in table]
table.update({key: example["jars"][key] for key in munit})

version = subprocess.run([teq, "--version"], capture_output=True, text=True, check=True).stdout.split()[1]
flags = {"ignoredScalacOptions": [], "kindProjector": False, "maxInlines": 32, "strictEquality": False, "werror": False}
def description(name, sources, state):
    return {"cacheableState": state, "excludes": [], "hot": False, "keys": {}, "macroState": "ordered", "modulePerFile": [], "out": name + "/target/teq/out",
            "production": {"excludes": [], "sources": []}, "release": False, "sources": sources}
# A JavaScript resident reads the Scala.js artifacts of its class path alone (`Export::jars_keyed`): the frontend's
# three libraries as their `_sjs1_3` jars, where bench/programs.sh checks it against the JVM jars' TASTy.
sjs = []
for path in (jar_of("cats-kernel-sjs"), jar_of("cats-core-sjs"), sorted(__import__("glob").glob(os.path.join(cache_root, "com/lihaoyi/sourcecode_sjs1_3/*/sourcecode_sjs1_3-*.jar")))[-1]):
    key, record = artifact(path)
    table[key] = record
    sjs.append(key)
def project(name, platform, sources, state, test=None):
    own = sjs if platform == "js" else jars
    configurations = {"compile": {"classpath": own, "flags": flags, "generators": [], "mainClasses": [], "resources": [], "sources": sources},
                      "runtime": {"classpath": [{"configuration": "compile", "project": name}] + own, "resources": [], "sources": []}}
    if test:
        configurations["test"] = {"classpath": [{"configuration": "compile", "project": name}] + jars + munit, "flags": flags, "resources": [], "sources": test,
                                  "baseDirectory": ".", "envVars": {}, "exclude": [], "fork": False, "javaOptions": [],
                                  "frameworks": [{"arguments": [], "class": "munit.Framework"}]}
    return {"base": name, "platform": platform, "scalaVersion": "3.8.4", "description": description(name, sources, state), "configurations": configurations}
export = {
    "teq": version, "format": 1, "binaries": {},
    "inputs": {"files": {}, "sha256": hashlib.sha256(b"").hexdigest()},
    "java": {"outputVersion": 17},
    "projects": {"web": project("web", "js", ["shared", "frontend"], ["meridian.web.css.Catalog"]),
                 "api": project("api", "jvm", ["shared", "api"], [], test=["apitest"])},
    "jars": table,
    "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}],
}
open(os.path.join(work, "teq.lock"), "w", encoding="utf-8").write(lock.canonical(export))
print(work)
