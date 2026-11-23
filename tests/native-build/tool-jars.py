#!/usr/bin/env python3
# tool-jars.py <directory>: jars written by the real tools that write Scala Native libraries, for
# jar-names-oracle.scala to judge (its second argument): the JDK's `jar` (create; -0; a manifest and none; update,
# once and twice; --date; --release 11; -i; many entries), its ZipOutputStream and JarOutputStream
# (tests/native-build/ZipWriter.java: stored with sizes, deflated with data descriptors, levels 1 and 9, comments,
# directory entries, a manifest), Info-ZIP's `zip` (-0, -1, -6, -9, -X, -D, a comment, -fz for zip64, streamed, a
# stub then -A, symbolic links followed and kept (-y), bzip2, split and joined and its last part alone, a name in
# Latin-1, a UTF-8 name under the C locale, many entries), Python's zipfile (stored, deflated, force_zip64, a comment,
# appended to, a 1980 date with a directory entry, bzip2, LZMA, many entries), 7-Zip (deflate, store, Deflate64, UTF-8
# names marked, the most passes), and the build tools that publish libraries, each on a project whose
# resources hold the files: sbt's packageBin, Mill's jar, Maven's jar plugin and Gradle's jar task, each by default and
# in its reproducible mode where it has one. Every jar holds the entry shapes a library's C code takes: a C file, a
# header in a subdirectory, an assembly file, the descriptor, a name with a blank, a name not ASCII, an empty file, a
# file over 64 KiB, the directory entries the tool writes; the `many` jars 600 entries more. Each jar is named
# `tool-<tool>-<variant>.jar`; `provenance.txt` beside them gives each one's tool, version and command. The tools are
# found on the PATH, or by JAR, JAVA, ZIP, SEVENZIP, SBT, MILL, MVN and GRADLE; a missing one stops the run. Run on a
# machine that has them, from the repository's root, under a UTF-8 locale.
import os, shutil, subprocess, sys, zipfile

out = os.path.abspath(sys.argv[1])
here = os.path.dirname(os.path.abspath(__file__))
work = os.path.join(out, "work")
shutil.rmtree(out, ignore_errors=True)
os.makedirs(work)
tool = {name: os.environ.get(name.upper().replace("-", ""), default) for name, default in
        [("jar", "jar"), ("java", "java"), ("zip", "zip"), ("sevenzip", "7z"), ("sbt", "sbt"), ("mill", "mill"), ("mvn", "mvn"), ("gradle", "gradle")]}
provenance = []


def run(args, cwd=None, timeout=1200):
    done = subprocess.run(args, cwd=cwd, capture_output=True, text=True, timeout=timeout)
    if done.returncode != 0:
        sys.exit(f"tool-jars: {' '.join(args)} exited {done.returncode}\n{done.stdout[-2000:]}{done.stderr[-2000:]}")
    return done.stdout + done.stderr


def version(args):
    return " ".join(run(args).split())[:120]


def tree(root, tag, many=False):
    """The entry shapes under root/scala-native (and the many entries under root/data)."""
    c = tag.replace("-", "_")
    files = {
        f"scala-native/{c}.c": f"int tw_{c}(void) {{ return 0; }}\n",
        f"scala-native/inc/{c}.h": f"#define TW_{c.upper()} 1\n",
        f"scala-native/{c}_asm.S": f".text\n.globl tw_{c}_asm\ntw_{c}_asm:\n\tret\n",
        "scala-native/scala-native.properties": f"compile.c.options = -DTW_{c.upper()}_DESCRIPTOR=1\n",
        f"scala-native/with blank {c}.c": f"int tw_{c}_blank(void) {{ return 0; }}\n",
        f"scala-native/ünï_{c}.c": f"int tw_{c}_unicode(void) {{ return 0; }}\n",
        f"scala-native/empty_{c}.h": "",
        f"scala-native/big_{c}.c": f"int tw_{c}_big(void) {{ return 0; }}\n/*\n" + "".join(f"{i:05d} a line of the big file\n" for i in range(3000)) + "*/\n",
    }
    if many:
        files.update({f"data/f{i:04d}.txt": f"{tag} {i}\n" for i in range(600)})
    for name, text in files.items():
        path = os.path.join(root, name)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
    return root


def made(name, which, command):
    provenance.append(f"{name} | {which} | {command}")


def fresh(tag, many=False):
    return tree(os.path.join(work, tag, "files"), tag, many)


# The JDK's jar tool.
jar_version = version([tool["jar"], "--version"])
for variant, flags in [("create", "cf"), ("stored", "cf0"), ("many", "cf")]:
    tag = f"jar-{variant}"
    files = fresh(tag, variant == "many")
    run([tool["jar"], flags, os.path.join(out, f"tool-{tag}.jar"), "-C", files, "."])
    made(f"tool-{tag}", jar_version, f"jar {flags} <jar> -C <files> .")
files = fresh("jar-manifest")
manifest = os.path.join(work, "jar-manifest", "MANIFEST.MF")
with open(manifest, "w") as f:
    f.write("Manifest-Version: 1.0\nImplementation-Title: tool-jars\n")
run([tool["jar"], "cfm", os.path.join(out, "tool-jar-manifest.jar"), manifest, "-C", files, "."])
made("tool-jar-manifest", jar_version, "jar cfm <jar> <manifest> -C <files> .")
files = fresh("jar-update")
first = os.path.join(work, "jar-update", "first")
os.makedirs(first)
shutil.move(os.path.join(files, "scala-native", "inc"), os.path.join(first, "inc"))
run([tool["jar"], "cf", os.path.join(out, "tool-jar-update.jar"), "-C", files, "."])
os.makedirs(os.path.join(work, "jar-update", "later", "scala-native"))
shutil.move(os.path.join(first, "inc"), os.path.join(work, "jar-update", "later", "scala-native", "inc"))
run([tool["jar"], "uf", os.path.join(out, "tool-jar-update.jar"), "-C", os.path.join(work, "jar-update", "later"), "."])
made("tool-jar-update", jar_version, "jar cf <jar> -C <files without inc/> . then jar uf <jar> -C <inc/> .")
files = fresh("jar-date")
run([tool["jar"], "--create", "--date=2001-01-01T00:00:00Z", "--file", os.path.join(out, "tool-jar-date.jar"), "-C", files, "."])
made("tool-jar-date", jar_version, "jar --create --date=2001-01-01T00:00:00Z --file <jar> -C <files> .")
files = fresh("jar-nomanifest")
run([tool["jar"], "cfM", os.path.join(out, "tool-jar-nomanifest.jar"), "-C", files, "."])
made("tool-jar-nomanifest", jar_version, "jar cfM <jar> -C <files> .")
files = fresh("jar-update-twice")
jar_path = os.path.join(out, "tool-jar-update-twice.jar")
run([tool["jar"], "cf", jar_path, "-C", files, "."])
for round_ in (1, 2):
    later = os.path.join(work, "jar-update-twice", f"round{round_}")
    os.makedirs(os.path.join(later, "scala-native"))
    with open(os.path.join(later, "scala-native", "jar_update_twice.c"), "w") as f:
        f.write(f"int tw_jar_update_twice(void) {{ return {round_}; }}\n")
    if round_ == 2:
        with open(os.path.join(later, "scala-native", "added_jar_update_twice.c"), "w") as f:
            f.write("int tw_jar_update_twice_added(void) { return 0; }\n")
    run([tool["jar"], "uf", jar_path, "-C", later, "."])
made("tool-jar-update-twice", jar_version, "jar cf <jar> -C <files> . then jar uf <jar> twice (a C file replaced twice, one added)")
files = fresh("jar-release")
versioned = os.path.join(work, "jar-release", "v11")
os.makedirs(os.path.join(versioned, "scala-native"))
with open(os.path.join(versioned, "scala-native", "jar_release.c"), "w") as f:
    f.write("int tw_jar_release_v11(void) { return 11; }\n")
run([tool["jar"], "--create", "--file", os.path.join(out, "tool-jar-release.jar"), "-C", files, ".", "--release", "11", "-C", versioned, "."])
made("tool-jar-release", jar_version, "jar --create --file <jar> -C <files> . --release 11 -C <v11> . (META-INF/versions/11/scala-native/jar_release.c)")
files = fresh("jar-index")
run([tool["jar"], "cf", os.path.join(out, "tool-jar-index.jar"), "-C", files, "."])
run([tool["jar"], "-i", os.path.join(out, "tool-jar-index.jar")])
made("tool-jar-index", jar_version, "jar cf <jar> -C <files> . then jar -i <jar>")

# The JDK's ZipOutputStream and JarOutputStream.
java_version = version([tool["java"], "-version"])
for mode in ["stored", "deflated", "deflated-1", "deflated-9", "commented", "directories", "jar"]:
    tag = f"zos-{mode}"
    files = fresh(tag, mode == "deflated")
    run([tool["java"], os.path.join(here, "ZipWriter.java"), os.path.join(out, f"tool-{tag}.jar"), files, mode])
    made(f"tool-{tag}", java_version, f"java tests/native-build/ZipWriter.java <jar> <files> {mode}")

# Info-ZIP's zip.
zip_version = " ".join(run([tool["zip"], "-v"]).splitlines()[1].split())
for variant, flags in [("0", ["-0"]), ("1", ["-1"]), ("6", ["-6"]), ("9", ["-9"]), ("X", ["-X"]), ("D", ["-D"]), ("fz", ["-fz"]), ("many", [])]:
    tag = f"zip-{variant}"
    files = fresh(tag, variant == "many")
    run([tool["zip"], "-q", "-r"] + flags + [os.path.join(out, f"tool-{tag}.jar"), "."], cwd=files)
    made(f"tool-{tag}", zip_version, f"zip -q -r {' '.join(flags)} <jar> . (in <files>)".replace("  ", " "))
files = fresh("zip-comment")
run(["sh", "-c", f"echo 'a comment on the archive' | {tool['zip']} -q -r -z {os.path.join(out, 'tool-zip-comment.jar')} ."], cwd=files)
made("tool-zip-comment", zip_version, "zip -q -r -z <jar> . (the comment on stdin)")
files = fresh("zip-stream")
run(["sh", "-c", f"{tool['zip']} -q -r - . | cat > {os.path.join(out, 'tool-zip-stream.jar')}"], cwd=files)
made("tool-zip-stream", zip_version, "zip -q -r - . | cat > <jar> (streamed: data descriptors)")
files = fresh("zip-sfx")
plain_zip = os.path.join(work, "zip-sfx", "plain.zip")
run([tool["zip"], "-q", "-r", plain_zip, "."], cwd=files)
with open(os.path.join(out, "tool-zip-sfx.jar"), "wb") as f:
    f.write(b"#!/bin/sh\necho a self-extracting stub\nexit 0\n" + b"#" * 64 + b"\n")
    f.write(open(plain_zip, "rb").read())
run([tool["zip"], "-q", "-A", os.path.join(out, "tool-zip-sfx.jar")])
made("tool-zip-sfx", zip_version, "a shell stub, then the zip of <files>, then zip -A <jar> (the offsets adjusted)")
for variant, flags in [("links-followed", []), ("links-kept", ["-y"])]:
    tag = f"zip-{variant}"
    files = fresh(tag)
    c = tag.replace("-", "_")
    os.symlink(f"inc/{c}.h", os.path.join(files, "scala-native", f"link_{c}.h"))
    os.symlink("inc", os.path.join(files, "scala-native", f"incl_{c}"))
    run([tool["zip"], "-q", "-r"] + flags + [os.path.join(out, f"tool-{tag}.jar"), "."], cwd=files)
    made(f"tool-{tag}", zip_version, f"zip -q -r {' '.join(flags)} <jar> . (a link to a header and one to a directory)".replace("  ", " "))
files = fresh("zip-bzip2")
run([tool["zip"], "-q", "-r", "-Z", "bzip2", os.path.join(out, "tool-zip-bzip2.jar"), "."], cwd=files)
made("tool-zip-bzip2", zip_version, "zip -q -r -Z bzip2 <jar> .")
for variant in ["joined", "last"]:
    tag = f"zip-split-{variant}"
    files = fresh(tag)
    split_at = os.path.join(work, tag)
    with open(os.path.join(files, "scala-native", f"blob_{variant}.bin"), "wb") as f:
        f.write(os.urandom(200000))
    run([tool["zip"], "-q", "-r", "-s", "64k", os.path.join(split_at, "part.zip"), "."], cwd=files)
    if variant == "joined":
        run([tool["zip"], "-q", "-s", "0", os.path.join(split_at, "part.zip"), "--out", os.path.join(split_at, "joined.zip")])
        shutil.copy(os.path.join(split_at, "joined.zip"), os.path.join(out, f"tool-{tag}.jar"))
        made(f"tool-{tag}", zip_version, "zip -q -r -s 64k <part.zip> . (200 kB of random bytes among the files) then zip -q -s 0 <part.zip> --out <joined.zip>")
    else:
        shutil.copy(os.path.join(split_at, "part.zip"), os.path.join(out, f"tool-{tag}.jar"))
        made(f"tool-{tag}", zip_version, "zip -q -r -s 64k <part.zip> . (200 kB of random bytes among the files): its last part alone")
files = fresh("zip-latin1")
open(os.path.join(files.encode(), b"scala-native", b"caf\xe9_zip_latin1.c"), "wb").write(b"int tw_zip_latin1(void) { return 0; }\n")
run(["env", "LC_ALL=C", tool["zip"], "-q", "-r", os.path.join(out, "tool-zip-latin1.jar"), "."], cwd=files)
made("tool-zip-latin1", zip_version, "LC_ALL=C zip -q -r <jar> . (a file named in Latin-1)")
files = fresh("zip-utf8-c-locale")
run(["env", "LC_ALL=C", tool["zip"], "-q", "-r", os.path.join(out, "tool-zip-utf8-c-locale.jar"), "."], cwd=files)
made("tool-zip-utf8-c-locale", zip_version, "LC_ALL=C zip -q -r <jar> . (the UTF-8 names under the C locale)")

# Python's zipfile.
python_version = f"Python {sys.version.split()[0]} zipfile"
for variant in ["stored", "deflated", "force-zip64", "commented", "many"]:
    tag = f"python-{variant}"
    files = fresh(tag, variant == "many")
    method = zipfile.ZIP_STORED if variant == "stored" else zipfile.ZIP_DEFLATED
    with zipfile.ZipFile(os.path.join(out, f"tool-{tag}.jar"), "w", method) as z:
        if variant == "commented":
            z.comment = b"a comment on the archive"
        for d, _, names in sorted(os.walk(files)):
            for n in sorted(names):
                p = os.path.join(d, n)
                arc = os.path.relpath(p, files)
                if variant == "force-zip64":
                    info = zipfile.ZipInfo.from_file(p, arc)
                    info.compress_type = method
                    with open(p, "rb") as src, z.open(info, "w", force_zip64=True) as dst:
                        dst.write(src.read())
                else:
                    z.write(p, arc)
    made(f"tool-{tag}", python_version, f"zipfile.ZipFile(<jar>, 'w', {'ZIP_STORED' if method == zipfile.ZIP_STORED else 'ZIP_DEFLATED'})"
         + (", force_zip64 on each entry" if variant == "force-zip64" else "") + (", a comment" if variant == "commented" else ""))
files = fresh("python-append")
appended = os.path.join(out, "tool-python-append.jar")
run([tool["jar"], "cf", appended, "-C", files, "."])
with zipfile.ZipFile(appended, "a", zipfile.ZIP_DEFLATED) as z:
    z.writestr("scala-native/appended_python_append.c", "int tw_python_append_added(void) { return 0; }\n")
made("tool-python-append", f"{jar_version}, then {python_version}", "jar cf <jar> -C <files> . then zipfile.ZipFile(<jar>, 'a') adds a C file")
files = fresh("python-pastdate")
with zipfile.ZipFile(os.path.join(out, "tool-python-pastdate.jar"), "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr(zipfile.ZipInfo("scala-native/", (1980, 1, 1, 0, 0, 0)), "")
    for d, _, names in sorted(os.walk(files)):
        for n in sorted(names):
            p = os.path.join(d, n)
            info = zipfile.ZipInfo(os.path.relpath(p, files), (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, open(p, "rb").read())
made("tool-python-pastdate", python_version, "zipfile ZipInfo entries dated 1980-01-01 and a directory entry")
for variant, method in [("bzip2", zipfile.ZIP_BZIP2), ("lzma", zipfile.ZIP_LZMA)]:
    tag = f"python-{variant}"
    files = fresh(tag)
    with zipfile.ZipFile(os.path.join(out, f"tool-{tag}.jar"), "w", method) as z:
        for d, _, names in sorted(os.walk(files)):
            for n in sorted(names):
                z.write(os.path.join(d, n), os.path.relpath(os.path.join(d, n), files))
    made(f"tool-{tag}", python_version, f"zipfile.ZipFile(<jar>, 'w', ZIP_{variant.upper()})")

# 7-Zip.
seven_version = " ".join(run([tool["sevenzip"]]).split("\n")[1].split()[:3])
for variant, flags in [("deflate", []), ("store", ["-mx=0"]), ("deflate64", ["-mm=Deflate64"]), ("utf8", ["-mcu=on"]), ("passes", ["-mx=9", "-mpass=15"])]:
    tag = f"7z-{variant}"
    files = fresh(tag)
    run([tool["sevenzip"], "a", "-tzip", "-bd"] + flags + [os.path.join(out, f"tool-{tag}.jar"), "."], cwd=files)
    made(f"tool-{tag}", seven_version, f"7z a -tzip {' '.join(flags)} <jar> . (in <files>)".replace("  ", " "))


def resources(project, tag, at):
    tree(os.path.join(project, at), tag)


# sbt's packageBin: by default (its fixed timestamps) and keeping the files' times.
for variant, setting in [("default", ""), ("keep-times", "ThisBuild / packageTimestamp := Package.keepTimestamps\n")]:
    tag = f"sbt-{variant}"
    project = os.path.join(work, tag, "project")
    os.makedirs(os.path.join(project, "project"))
    with open(os.path.join(project, "project", "build.properties"), "w") as f:
        f.write("sbt.version=1.10.7\n")
    with open(os.path.join(project, "build.sbt"), "w") as f:
        f.write(f'name := "tw"\nversion := "0.1"\ncrossPaths := false\nautoScalaLibrary := false\n{setting}')
    resources(project, tag, "src/main/resources")
    run([tool["sbt"], "-batch", "-Dsbt.server.forcestart=false", "package"], cwd=project, timeout=1800)
    shutil.copy(os.path.join(project, "target", "tw-0.1.jar"), os.path.join(out, f"tool-{tag}.jar"))
    made(f"tool-{tag}", "sbt 1.10.7", "sbt package" + (" with packageTimestamp := Package.keepTimestamps" if setting else ""))

# Mill's jar.
tag = "mill-default"
project = os.path.join(work, tag, "project")
os.makedirs(project)
with open(os.path.join(project, "build.mill"), "w") as f:
    f.write("//| mill-version: 1.3.1\npackage build\nimport mill.*, javalib.*\nobject tw extends JavaModule\n")
resources(project, tag, "tw/resources")
run([tool["mill"], "--no-server", "tw.jar"], cwd=project, timeout=1800)
shutil.copy(os.path.join(project, "out", "tw", "jar.dest", "out.jar"), os.path.join(out, f"tool-{tag}.jar"))
made(f"tool-{tag}", "Mill 1.3.1", "mill tw.jar")

# Maven's jar plugin: by default and with project.build.outputTimestamp (its reproducible mode).
mvn_version = " ".join(run([tool["mvn"], "-v"]).splitlines()[0].split()[:3])
for variant, timestamp in [("default", ""), ("reproducible", "<project.build.outputTimestamp>2025-01-01T00:00:00Z</project.build.outputTimestamp>")]:
    tag = f"maven-{variant}"
    project = os.path.join(work, tag, "project")
    os.makedirs(project)
    with open(os.path.join(project, "pom.xml"), "w") as f:
        f.write(f"""<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion>
<groupId>tw</groupId><artifactId>tw</artifactId><version>0.1</version><packaging>jar</packaging>
<properties><project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>{timestamp}</properties>
<build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-jar-plugin</artifactId><version>3.4.2</version></plugin></plugins></build>
</project>
""")
    resources(project, tag, "src/main/resources")
    run([tool["mvn"], "-B", "-q", "package"], cwd=project, timeout=1800)
    shutil.copy(os.path.join(project, "target", "tw-0.1.jar"), os.path.join(out, f"tool-{tag}.jar"))
    made(f"tool-{tag}", f"{mvn_version}, maven-jar-plugin 3.4.2", "mvn -B -q package" + (", project.build.outputTimestamp set" if timestamp else ""))

# Gradle's jar task: by default and with its archive tasks' reproducible settings.
gradle_version = " ".join(l for l in run([tool["gradle"], "--version", "--no-daemon"]).splitlines() if l.startswith("Gradle "))
for variant, settings in [("default", ""), ("reproducible", "tasks.withType(AbstractArchiveTask).configureEach { preserveFileTimestamps = false; reproducibleFileOrder = true }\n")]:
    tag = f"gradle-{variant}"
    project = os.path.join(work, tag, "project")
    os.makedirs(project)
    with open(os.path.join(project, "settings.gradle"), "w") as f:
        f.write("rootProject.name = 'tw'\n")
    with open(os.path.join(project, "build.gradle"), "w") as f:
        f.write("plugins { id 'java' }\nversion = '0.1'\n" + settings)
    resources(project, tag, "src/main/resources")
    run([tool["gradle"], "-q", "--no-daemon", "jar"], cwd=project, timeout=1800)
    shutil.copy(os.path.join(project, "build", "libs", "tw-0.1.jar"), os.path.join(out, f"tool-{tag}.jar"))
    made(f"tool-{tag}", gradle_version, "gradle jar" + (" with preserveFileTimestamps = false, reproducibleFileOrder = true" if settings else ""))

shutil.rmtree(work)
with open(os.path.join(out, "provenance.txt"), "w") as f:
    f.write("\n".join(provenance) + "\n")
print(f"tool-jars: {len(provenance)} jars in {out}")
