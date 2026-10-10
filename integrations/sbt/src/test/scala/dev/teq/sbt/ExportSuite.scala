package dev.teq.sbt

import java.io.File
import java.net.{InetAddress, InetSocketAddress}
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.Files
import java.security.MessageDigest
import java.time.Duration
import java.util.concurrent.{CountDownLatch, Executors, TimeUnit}
import scala.collection.mutable
import scala.collection.compat.*

import com.sun.net.httpserver.{BasicAuthenticator, HttpServer}
import sbt.io.IO
import sbt.librarymanagement.{MavenRepository, Resolver}
import sbt.util.{Level, Logger}

import Compat.Credentials

class ExportSuite extends munit.FunSuite {
  private def sha256(text: String) = MessageDigest.getInstance("SHA-256").digest(text.getBytes(UTF_8)).map(b => f"${b & 0xff}%02x").mkString

  private def withDirectory[A](body: File => A): A = {
    val dir = Files.createTempDirectory("teq-export").toFile
    try body(dir) finally IO.delete(dir)
  }

  /** Whether a directory lies in a Git work tree the test does not own: a `.git` at it or above, as a machine's
    * temporary directory may have one. A test of a build in no work tree states its answer under this condition. */
  private def inWorkTree(dir: File): Boolean =
    Iterator.iterate(dir.getAbsoluteFile)(_.getParentFile).takeWhile(_ != null).exists(d => new File(d, ".git").exists)

  /** The conformance corpus of `tests/lock/`, which teq's writer, check-export.py's and this one
    * write to the same bytes and the YAML 1.2 core reader reads to `corpus.json`'s tree. */
  private def corpus(name: String): String = {
    val dir = Iterator.iterate(new File(".").getCanonicalFile)(_.getParentFile).takeWhile(_ != null).map(new File(_, "tests/lock")).find(_.isDirectory)
    IO.read(new File(dir.getOrElse(sys.error("no tests/lock above the working directory")), name), UTF_8)
  }

  test("the lock's canonical form: the conformance corpus's tree written to its bytes") {
    assertEquals(Lock.canonical(Json.parse(corpus("corpus.json"))), corpus("corpus.lock"))
  }

  test("the lock's canonical form: the header first, a record of a classpath or a layer on one line unless it holds a list, the jar table's fields") {
    def obj(fields: (String, Json.Value)*) = Json.Obj(fields.toMap)
    def str(s: String) = Json.Str(s)
    val value = obj(
      "projects" -> obj("p" -> obj(
        "configurations" -> obj("compile" -> obj("classpath" -> Json.Arr(Seq(obj("project" -> str("q"), "configuration" -> str("compile")), str("a:b:1"), obj("file" -> str("lib/x y.jar")))))),
        "stage" -> obj("layers" -> obj("2" -> Json.Arr(Seq(
          obj("from" -> obj("project" -> str("p"), "configuration" -> str("compile")), "manifest" -> obj("Main-Class" -> str("M")), "to" -> str("opt/docker/lib/p.jar")),
          obj("script" -> str("opt/docker/bin/p"), "classpath" -> Json.Arr(Seq(str("lib/a.jar")))),
        )))),
        "generators" -> Json.Arr(Seq(obj("kind" -> str("command"), "run" -> Json.Arr(Seq(str("sh"), str("-c"))), "dynamic" -> obj("x" -> obj("w" -> str("z")))))),
      )),
      "lists" -> Json.Arr(Seq(Json.Arr(Seq(str("a"), str("b"))), Json.Num("3"))),
      "binaries" -> obj("linux-x86_64" -> str("https://r/teq.exe ff15f2278065734035de325534184128c33eeba1 3")),
      "format" -> Json.Num("1"),
      "teq" -> str("1.0"),
    )
    // The same document as task::lock's blocks_lists_and_records in teq.
    assertEquals(
      Lock.canonical(value),
      """teq: "1.0"
        |format: 1
        |binaries:
        |  linux-x86_64: https://r/teq.exe ff15f2278065734035de325534184128c33eeba1 3
        |lists:
        |  - - a
        |    - b
        |  - 3
        |projects:
        |  p:
        |    configurations:
        |      compile:
        |        classpath:
        |          - {configuration: compile, project: q}
        |          - a:b:1
        |          - {file: "lib/x y.jar"}
        |    generators:
        |      - dynamic:
        |          x:
        |            w: z
        |        kind: command
        |        run:
        |          - sh
        |          - "-c"
        |    stage:
        |      layers:
        |        "2":
        |          - {from: {configuration: compile, project: p}, manifest: {Main-Class: M}, to: opt/docker/lib/p.jar}
        |          - classpath:
        |              - lib/a.jar
        |            script: opt/docker/bin/p
        |""".stripMargin,
    )
  }

  test("the quoting predicate") {
    for (s <- Seq("a", "3.8.4", "org.typelevel:cats-core_3:2.13.0", "https://repo1.maven.org/maven2/", "ff15f2278065734035de325534184128c33eeba1", ".gitignore", "1.0@build")) assert(Lock.plain(s), s)
    for (s <- Seq("", "true", "yes", "on", "Null", "2", "1.0", "1e3", ".5", "2026-10-05", "10:30", "0x1F", "0b101", "-Xmx1g", "a:", "a b", "é", "a#b", "~")) assert(!Lock.plain(s), s)
    assert(Lock.plainLine("maven-central ff15f2278065734035de325534184128c33eeba1 12"))
    for (s <- Seq("2 a", "a  b", "a ", "a -b")) assert(!Lock.plainLine(s), s)
    assertEquals(Lock.quoted("a\"b\\c\nd\re\tf\u0001\u007f\u0085\u2028\ufeffé😀"), "\"a\\\"b\\\\c\\nd\\re\\tf\\u0001\\u007f\\u0085\\u2028\\ufeffé😀\"")
  }

  test("what the lock cannot hold: a key past YAML's 1,024 characters, a lone surrogate") {
    val long = "k" * 1025
    val problems = Lock.problems(Json.Obj(Map("projects" -> Json.Obj(Map("p" -> Json.Obj(Map("description" -> Json.Obj(Map("keys" -> Json.Obj(Map(long -> Json.Str("v"), "ok" -> Json.Str("a\ud800b"))))))))))))
    assertEquals(problems.size, 2, problems)
    assert(problems.head.startsWith("the key kkk") && problems.head.contains("at projects.p.description.keys is 1025 characters as teq.lock writes it, more than YAML's 1024 for a key"), problems.head)
    assert(problems(1).contains("at projects.p.description.keys.ok holds a lone surrogate"), problems(1))
    assertEquals(Lock.problems(Json.Obj(Map(("k" * 1024) -> Json.Str("v"), "a" -> Json.Str("😀")))), Nil)
  }

  test("a setting's position names a file of the build only where it is a path under the root that exists") {
    val root = Files.createTempDirectory("teq-positions").toRealPath()
    try {
      Files.writeString(root.resolve("build.sbt"), "name := \"x\"\n")
      val outside = Files.createTempFile("teq-outside", ".sbt").toRealPath()
      try {
        assert(Export.inBuildFile(sbt.internal.util.LinePosition("build.sbt", 1), root))
        assert(Export.inBuildFile(sbt.internal.util.LinePosition(root.resolve("build.sbt").toString, 1), root))
        assert(!Export.inBuildFile(sbt.internal.util.LinePosition(outside.toString, 1), root))
        assert(!Export.inBuildFile(sbt.internal.util.LinePosition("missing.sbt", 1), root))
        // A plugin's setting carries its source text as the position's path; Windows refuses such a string as a
        // path (a colon), every platform a NUL character: no file of the build either way.
        assert(!Export.inBuildFile(sbt.internal.util.LinePosition("dockerGroupLayers := {\n  val conv0 = fileConverter.value\n}", 1), root))
        assert(!Export.inBuildFile(sbt.internal.util.LinePosition("build\u0000.sbt", 1), root))
        assert(!Export.inBuildFile(sbt.internal.util.NoPosition, root))
      }
      finally Files.deleteIfExists(outside)
    }
    finally IO.delete(root.toFile)
  }

  test("a session's set of a key in a scope is the build's declaration, for that key, scope and project alone") {
    import sbt.*
    import sbt.Keys.{mainClass, run}
    val ref = ProjectRef(new File("/b").toURI, "api")
    val other = ProjectRef(new File("/b").toURI, "jvmapp")
    val compile = Scope(Select(ref), Select(ConfigKey("compile")), Zero, Zero)
    // As sbt appends a `set`: its scope's axes resolved (the task and extra axes Zero), its position the session's.
    val setting: Def.Setting[?] = Def.setting(Def.ScopedKey(compile, mainClass.key), Def.task(Option.empty[String]), sbt.internal.util.LinePosition("<set>", 0))
    assert(Export.setBySession(Seq(setting), compile, mainClass.key))
    assert(!Export.setBySession(Seq(setting), compile.copy(task = Select(run.key)), mainClass.key))
    assert(!Export.setBySession(Seq(setting), compile.copy(project = Select(other)), mainClass.key))
    assert(!Export.setBySession(Seq(setting), compile, Keys.name.key))
    assert(!Export.setBySession(Nil, compile, mainClass.key))
    // A `set every` is stored as one setting per scope where the key is defined, each project's own;
    // a `set` of a scope no project's lookup reaches (`Zero / Compile / mainClass`) sets nothing.
    val every: Seq[Def.Setting[?]] = Seq(compile, compile.copy(project = Select(other))).map(scope => Def.setting(Def.ScopedKey(scope, mainClass.key), Def.task(Option.empty[String]), sbt.internal.util.LinePosition("<set>", 0)))
    assert(Export.setBySession(every, compile, mainClass.key))
    assert(Export.setBySession(every, compile.copy(project = Select(other)), mainClass.key))
    assert(!Export.setBySession(every, compile.copy(task = Select(run.key)), mainClass.key))
    val unreached: Def.Setting[?] = Def.setting(Def.ScopedKey(compile.copy(project = Zero), mainClass.key), Def.task(Option.empty[String]), sbt.internal.util.LinePosition("<set>", 0))
    assert(!Export.setBySession(Seq(unreached), compile, mainClass.key))
  }

  test("a jar's key and the Maven layout a reader derives from it") {
    assertEquals(Export.jarKey("org.jline", "jline", "3.29.0", Some("jdk8")), "org.jline:jline:3.29.0:jdk8")
    assertEquals(Export.jarKey("a", "b", "1", Some("")), "a:b:1")
    assertEquals(Export.mavenLayout("org.jline:jline:3.29.0:jdk8"), Some("org/jline/jline/3.29.0/jline-3.29.0-jdk8.jar"))
    assertEquals(Export.mavenLayout("org.scala-lang:scala3-library_3:3.8.4"), Some("org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar"))
    for (no <- Seq("a:b", "a:b:c:d:e", "a::1", "a:b:1:", "")) assertEquals(Export.mavenLayout(no), None, no)
    // A path off the layout (a timestamped snapshot) is written as the record's fourth field; one on it is derived.
    val r = Export.Repository("r", "https://r.example/", Some("r.example"))
    val snapshot = Export.Jar("x:snap:1.0-SNAPSHOT", r, "x/snap/1.0-SNAPSHOT/snap-1.0-20261005.101010-3.jar", "s", 1, "p/compile")
    assertEquals(snapshot.fields(_.id), Json.Str(s"r s 1 ${snapshot.path}"))
    assertEquals(Export.Jar("x:y:1.0", r, "x/y/1.0/y-1.0.jar", "s", 1, "p/compile").fields(_.id), Json.Str("r s 1"))
    // A path with a space is refused: the fields could not hold it.
    assertEquals(Export.table(Seq(Export.Jar("x:y:1.0", r, "x/y/1.0/y 1.0.jar", "s", 1, "p/compile")))._2.size, 1)
  }

  test("one key, one jar: a key that names two files is refused with both") {
    val central = Export.Repository("maven-central", "https://repo1.maven.org/maven2/", None)
    val company = Export.Repository("company", "https://artifacts.example.io/releases/", Some("artifacts.example.io"))
    val jar = Export.Jar("a:b:1", central, "a/b/1/b-1.jar", "da39a3ee5e6b4b0d3255bfef95601890afd80709", 10, "p/compile")
    // One repository under two projects' names is one: q's resolver names company's URL "second".
    val aliased = Export.Jar("a:c:1", company, "a/c/1/c-1.jar", "s", 1, "p/compile")
    val (one, none) = Export.table(Seq(jar, jar.copy(where = "q/test"), aliased, aliased.copy(repository = company.copy(id = "second"), where = "q/compile")))
    assertEquals((one.map(_.key), none), (Seq("a:b:1", "a:c:1"), Nil))
    for (other <- Seq(jar.copy(repository = company, where = "q/test"), jar.copy(path = "a/b/1/b-1-x.jar", where = "q/test"), jar.copy(sha1 = "0" * 40, where = "q/test"), jar.copy(size = 11, where = "q/test"))) {
      val (_, refusals) = Export.table(Seq(jar, other))
      assertEquals(refusals, Seq(s"the jar a:b:1 is two files, $jar and $other: one key names one jar"))
      assert(refusals.head.contains("for p/compile") && refusals.head.contains("for q/test"), refusals.head)
    }
  }

  test("one repository per URL and credentials host, by the first name a project gives it") {
    val url = "http://127.0.0.1:38443/"
    val (first, second) = (Export.Repository("first", url, Some("127.0.0.1")), Export.Repository("second", url, Some("127.0.0.1")))
    val central = Export.Repository("maven-central", "https://repo1.maven.org/maven2/", None)
    val (repositories, id, conflicts) = Export.shared(Seq(central, first, central, second))
    assertEquals((repositories, Seq(central, first, second).map(id), conflicts), (Seq(central, first), Seq("maven-central", "first", "first"), Nil))
    // One name for two URLs is refused, as before.
    val other = Export.Repository("first", "https://elsewhere.example/", Some("elsewhere.example"))
    assertEquals(Export.shared(Seq(first, other))._3, Seq(s"the repository id first names $url and https://elsewhere.example/: give the resolvers names of their own"))
  }

  test("the build definition files and the hash over them") {
    withDirectory { root =>
      for ((path, text) <- Seq(
          "build.sbt" -> "a",
          "other.sbt" -> "b",
          "notes.txt" -> "n",
          "project/build.properties" -> "sbt.version=2.0.8\n",
          "project/D.scala" -> "d",
          "project/project/P.scala" -> "p",
          "project/target/x/T.scala" -> "t",
          "project/.bsp/B.scala" -> "b",
          "project/notes.txt" -> "n",
          "sub/inner.sbt" -> "i",
        ))
      IO.write(new File(root, path), text, UTF_8)
      val (files, hash) = Export.inputs(root.toPath)
      val expected = Seq(
        "build.sbt" -> sha256("a"),
        "other.sbt" -> sha256("b"),
        "project/D.scala" -> sha256("d"),
        "project/build.properties" -> sha256("sbt.version=2.0.8\n"),
        "project/project/P.scala" -> sha256("p"),
      )
      assertEquals(files, expected)
      assertEquals(hash, sha256(expected.map{ case (f, d) => s"${f}\u0000${d}\n"}.mkString))
    }
  }

  test("a build file with CRLF: its bytes digested as they are, and the export's warning naming it") {
    withDirectory { root =>
      val string = "object S { val s = \"\"\"a\nb\"\"\" }\n"
      for ((path, text) <- Seq(
          "build.sbt" -> "name := \"x\"\r\n",
          "other.sbt" -> "name := \"y\"\n",
          "project/S.scala" -> string.replace("\n", "\r\n"),
          "project/build.properties" -> "sbt.version=2.0.8\rmore\n",
        ))
      IO.write(new File(root, path), text, UTF_8)
      // The raw bytes, CRLF and all: scalac keeps the `\r\n` of S's multi-line string.
      assertEquals(Export.inputs(root.toPath)._1.toMap, Map(
        "build.sbt" -> sha256("name := \"x\"\r\n"),
        "other.sbt" -> sha256("name := \"y\"\n"),
        "project/S.scala" -> sha256(string.replace("\n", "\r\n")),
        "project/build.properties" -> sha256("sbt.version=2.0.8\rmore\n"),
      ))
      // A lone `\r` is no CRLF. The directory is in no Git work tree: no checkout to correct, so
      // the export on the other copy's machine; in one above the temporary directory, the remedy of .gitattributes.
      val above = inWorkTree(root)
      assertEquals(Export.crlfWarning(root.toPath), Some(
        if (above) s"teq: build.sbt, project/S.scala have CRLF line ends, and teq.lock records their bytes, so that a checkout with LF finds it stale: ${Export.LfRemedy}"
        else "teq: build.sbt, project/S.scala have CRLF line ends, and teq.lock records their bytes, so that a copy of the build that differs from them by line ends alone finds it stale until it is exported on that copy's machine"))
      IO.write(new File(root, "project/S.scala"), string, UTF_8)
      assertEquals(Export.crlfWarning(root.toPath), Some(
        if (above) s"teq: build.sbt has CRLF line ends, and teq.lock records its bytes, so that a checkout with LF finds it stale: ${Export.LfRemedy}"
        else "teq: build.sbt has CRLF line ends, and teq.lock records its bytes, so that a copy of the build that differs from it by line ends alone finds it stale until it is exported on that copy's machine"))
      // In a Git work tree, a repository's `.git` at the root, the remedy of .gitattributes.
      IO.createDirectory(new File(root, ".git"))
      assertEquals(Export.crlfWarning(root.toPath), Some(s"teq: build.sbt has CRLF line ends, and teq.lock records its bytes, so that a checkout with LF finds it stale: ${Export.LfRemedy}"))
      assert(Export.LfRemedy.endsWith(", then export again"), Export.LfRemedy)
      IO.write(new File(root, "build.sbt"), "name := \"x\"\n", UTF_8)
      assertEquals(Export.crlfWarning(root.toPath), None)
    }
  }

  test("the export's Git advice in a build of a Git work tree alone: a repository above it, a worktree's .git file") {
    withDirectory { dir =>
      val build = new File(dir, "repository/builds/b")
      IO.write(new File(build, "build.sbt"), "name := \"x\"\r\n", UTF_8)
      val git = s"teq: build.sbt has CRLF line ends, and teq.lock records its bytes, so that a checkout with LF finds it stale: ${Export.LfRemedy}"
      // The Git advice without a repository of the test's own only in a work tree above the temporary directory.
      assertEquals(Export.crlfWarning(build.toPath).contains(git), inWorkTree(dir))
      // A build below the repository's root.
      IO.createDirectory(new File(dir, "repository/.git"))
      assertEquals(Export.crlfWarning(build.toPath), Some(git))
      // A worktree, whose `.git` is a file naming the repository's.
      IO.delete(new File(dir, "repository/.git"))
      IO.write(new File(build, ".git"), "gitdir: /elsewhere/.git/worktrees/b\n", UTF_8)
      assertEquals(Export.crlfWarning(build.toPath), Some(git))
    }
  }

  test("the export's remedy for CRLF is the one teq's note gives") {
    val dir = Iterator.iterate(new File(".").getCanonicalFile)(_.getParentFile).takeWhile(_ != null).find(d => new File(d, "src/task/export.rs").isFile)
    val rust = IO.read(new File(dir.getOrElse(sys.error("no src/task/export.rs above the working directory")), "src/task/export.rs"), UTF_8)
    assert(rust.contains(s"""pub const LF_REMEDY: &str = "${Export.LfRemedy}";"""), "src/task/export.rs's LF_REMEDY differs from Export.LfRemedy")
  }

  test("the build's remote Maven repositories, named") {
    val repositories = Export.repositoriesOf(Seq(
      Resolver.defaultLocal,
      Resolver.DefaultMavenRepository,
      MavenRepository("Example Artifacts Releases", "https://artifacts.example.io/repository/maven-releases"),
      MavenRepository("example artifacts releases", "https://other.example.io/maven-releases/"),
      MavenRepository("again", "https://artifacts.example.io/repository/maven-releases/"),
      MavenRepository("local file", "file:///tmp/repository/"),
    ))
    assertEquals(repositories, Seq(
      Export.Repository("maven-central", "https://repo1.maven.org/maven2/", None),
      Export.Repository("example-artifacts-releases", "https://artifacts.example.io/repository/maven-releases/", Some("artifacts.example.io")),
      Export.Repository("example-artifacts-releases-2", "https://other.example.io/maven-releases/", Some("other.example.io")),
    ))
  }

  test("scalac's -Wunused kinds map to --wunused with every kind, the warning options to teq's") {
    def flags(options: String*) = TeqPlugin.ScalacOptions(options).flags(jvm = false)
    def ignored(options: String*) = TeqPlugin.ScalacOptions(options).ignored
    assertEquals(flags("-Wunused:imports"), Seq("--wunused", "imports"))
    assertEquals(flags("-Wunused:privates,imports", "-Wunused:locals"), Seq("--wunused", "privates,imports,locals"))
    assertEquals(flags("-Wunused"), Seq("--wunused", "all"))
    assertEquals(flags("-Wall"), Seq("--wunused", "all", "--wtostring-interpolated"))
    for (on <- Seq(Seq("-Wunused:imports"), Seq("-Wunused:all"), Seq("-Wunused"), Seq("-Wunused:privates,imports"), Seq("-Wunused:linted"), Seq("-Wall")))
      assert(TeqPlugin.ScalacOptions(on).wunusedImports, on)
    assert(!TeqPlugin.ScalacOptions(Seq("-Wunused:privates")).wunusedImports)
    assertEquals(ignored("-Wunused:privates,imports,locals"), Nil)
    assertEquals(ignored("-Wall"), Seq("-Wall"))
    assertEquals(flags("-Werror", "-Wunused:imports"), Seq("--werror", "--wunused", "imports"))
    assertEquals(
      flags("-deprecation", "-feature", "-Wtostring-interpolated", "-Wconf:id=E198:s", "-Wconf:cat=deprecation:e,any:w"),
      Seq("--deprecation", "--feature", "--wtostring-interpolated", "--wconf", "id=E198:s", "--wconf", "cat=deprecation:e,any:w"),
    )
    assertEquals(flags("-language:implicitConversions,strictEquality"), Seq("--strict-equality", "--language", "implicitConversions"))
    assertEquals(ignored("-deprecation", "-feature", "-Wconf:any:s", "-language:implicitConversions"), Nil)
  }

  test("dynamic versions") {
    for (version <- Seq("1.0.+", "latest.release", "[1.0,2.0)", "]1.0,)", "(,2.0]")) assert(Export.isDynamic(version), version)
    for (version <- Seq("1.0", "1.0-SNAPSHOT", "0.4.3-M5", "0.1.0-pre.2")) assert(!Export.isDynamic(version), version)
  }

  test("snapshot versions") {
    for (version <- Seq("0.1.0-SNAPSHOT", "0.1.1-releases-SNAPSHOT")) assert(Export.isSnapshot(version), version)
    for (version <- Seq("0.1.1", "0.1.0-pre.1", "1.0-SNAPSHOT-1")) assert(!Export.isSnapshot(version), version)
  }

  test("a file of coursier's cache by its URL") {
    val cache = new File("/c/coursier/v1")
    assertEquals(Export.cachedUrl(cache, new File("/c/coursier/v1/https/repo1.maven.org/maven2/a/b/1/b-1.jar")), Some("https://repo1.maven.org/maven2/a/b/1/b-1.jar"))
    assertEquals(Export.cachedUrl(cache, new File("/elsewhere/b-1.jar")), None)
    assertEquals(Export.cachedUrl(cache, new File("/c/coursier/v1/http/127.0.0.1%3A34563/a/b/1.0%2B3/b-1.0%2B3.jar")), Some("http://127.0.0.1:34563/a/b/1.0+3/b-1.0+3.jar"))
  }

  test("sbt's own scala-library is the jar of the declared repository the build's resolvers give it, never Central unasked") {
    val key = "org.scala-lang:scala-library:3.8.4"
    val path = "org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
    // A mirror alone, named as Central is: its jar, at its path; no second maven-central.
    val mirror = Export.Repository("maven-central", "http://127.0.0.1:33113/", Some("127.0.0.1"))
    assertEquals(Export.bootRepository(key, Right(Some(mirror.url + path)), Seq(mirror)), Right((mirror, path)))
    val named = mirror.copy(id = "mirror")
    assertEquals(Export.bootRepository(key, Right(Some(named.url + path)), Seq(named)), Right((named, path)))
    // Resolved from no declared repository, or not at all: refused, the jar and the resolvers named.
    val central = "https://repo1.maven.org/maven2/"
    for ((resolved, why) <- Seq(Right(Some(central + path)) -> s"it resolves from $central$path", Left("not found: scala-library") -> "not found: scala-library", Right(None) -> "")) {
      val refusal = Export.bootRepository(key, resolved, Seq(named)).left.toOption.getOrElse("")
      assert(refusal.startsWith(s"sbt's own $key, from its boot directory, is the jar of none of the build's Maven repositories (tried mirror (http://127.0.0.1:33113/))"), refusal)
      assert(refusal.endsWith(why), refusal)
    }
    assert(Export.bootRepository(key, Left("x"), Nil).left.toOption.exists(_.contains("(tried none)")))
  }

  /** A Maven repository on loopback serving a directory's files, a HEAD with the `Content-Length`
    * alone, each request recorded as `<method> <path>` (and ` (<range>)` for a GET's `Range`): `answer`
    * stands in for a path's file (a status, or a redirect to another path), `head` for a HEAD of a path (a
    * status), `stalled` paths answer 40 bytes' headers and one byte and then wait, `user` is the basic
    * authentication every request needs; `ranged` answers a GET's `bytes=<first>-<last>` with those bytes,
    * 206 and the `Content-Range`, and without it a range is ignored. */
  private def withRepository[A](dir: File, answer: String => Option[Either[Int, String]] = _ => None, user: Option[(String, String)] = None,
      stalled: String => Boolean = _ => false, head: String => Option[Int] = _ => None, ranged: Boolean = false)(body: (String, () => Seq[String]) => A): A = {
    val requests = mutable.ArrayBuffer.empty[String]
    val server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress, 0), 0)
    val handlers = Executors.newCachedThreadPool()
    val released = new CountDownLatch(1)
    server.setExecutor(handlers)
    val context = server.createContext("/", exchange => {
      val path = exchange.getRequestURI.getPath
      val range = Option(exchange.getRequestHeaders.getFirst("Range"))
      requests.synchronized(requests += s"${exchange.getRequestMethod} $path${range.fold("")(r => s" ($r)")}")
      val file = new File(dir, path.stripPrefix("/"))
      val bytes = """bytes=(\d+)-(\d+)""".r
      if (stalled(path)) {
        exchange.sendResponseHeaders(200, 40)
        exchange.getResponseBody.write('0')
        exchange.getResponseBody.flush()
        released.await(30, TimeUnit.SECONDS)
      }
      else if (exchange.getRequestMethod == "HEAD" && head(path).isDefined) exchange.sendResponseHeaders(head(path).get, -1)
      else answer(path) match {
        case Some(Left(status)) => exchange.sendResponseHeaders(status, -1)
        case Some(Right(to)) =>
          exchange.getResponseHeaders.set("Location", to)
          exchange.sendResponseHeaders(302, -1)
        case None if file.isFile && exchange.getRequestMethod == "HEAD" =>
          exchange.getResponseHeaders.set("Content-Length", file.length.toString)
          exchange.sendResponseHeaders(200, -1)
        case None if file.isFile && ranged && range.exists(bytes.pattern.matcher(_).matches) =>
          val (first, last) = range.get match { case bytes(f, l) => (f, l) }
          val slice = IO.readBytes(file).slice(first.toInt, last.toInt + 1)
          exchange.getResponseHeaders.set("Content-Range", s"bytes $first-$last/${file.length}")
          exchange.sendResponseHeaders(206, slice.length)
          exchange.getResponseBody.write(slice)
        case None if file.isFile =>
          exchange.sendResponseHeaders(200, file.length)
          exchange.getResponseBody.write(IO.readBytes(file))
        case None => exchange.sendResponseHeaders(404, -1)
      }
      try exchange.close() catch { case _: java.io.IOException => () }
    })
    for ((name, password) <- user)
      context.setAuthenticator(new BasicAuthenticator("teq") {
        def checkCredentials(u: String, p: String) = u == name && p == password
      })
    server.start()
    try body(s"http://127.0.0.1:${server.getAddress.getPort}/", () => requests.synchronized(requests.toSeq))
    finally {
      released.countDown()
      server.stop(0)
      handlers.shutdownNow()
    }
  }

  /** A release's files as its publish lays them out: the pom, and per classifier the binary and its
    * `.sha1` (the digest, then the file's name as sha1sum writes it, for the second). */
  private def published(dir: File, version: String, binaries: (String, String)*): Unit = {
    val at = new File(dir, s"build/teq/teq/$version")
    IO.write(new File(at, s"teq-$version.pom"), "<project/>")
    for (((classifier, bytes), i) <- binaries.zipWithIndex) {
      val name = s"teq-$version-$classifier.exe"
      IO.write(new File(at, name), bytes)
      IO.write(new File(at, s"$name.sha1"), Sha1.compute(new File(at, name)) + (if (i == 1) s"  $name\n" else ""))
    }
  }

  private final class Recorded extends Logger {
    val lines = mutable.ArrayBuffer.empty[String]
    def trace(t: => Throwable): Unit = ()
    def success(message: => String): Unit = ()
    def log(level: Level.Value, message: => String): Unit = lines += s"$level: $message"
  }

  private def repository(id: String, url: String) = Export.Repository(id, url, Some("127.0.0.1"))
  private val unserved = Served(Nil, Logger.Null)

  test("the binaries' records from the first repository serving the version by its pom: the sha1 from the .sha1, the size from a HEAD, none downloaded") {
    withDirectory { root =>
      val (orphan, releases, later) = (new File(root, "orphan"), new File(root, "releases"), new File(root, "later"))
      val at = "build/teq/teq/1.2.3"
      // The binaries without the pom, which coursier never resolves the module from; the module whole;
      // a later repository with the classifier the second lacks, which coursier never looks in.
      published(orphan, "1.2.3", "linux-x86_64" -> "other bytes!")
      IO.delete(new File(orphan, s"$at/teq-1.2.3.pom"))
      published(releases, "1.2.3", "linux-x86_64" -> "linux binary", "osx-aarch_64" -> "a mac binary")
      published(later, "1.2.3", "windows-x86_64" -> "windows binary")
      withRepository(orphan) { case (first, firstRequests) =>
        withRepository(releases) { case (second, requests) =>
          withRepository(later) { case (third, thirdRequests) =>
            val repositories = Seq(repository("first", first), repository("second", second), repository("third", third))
            val pins = Export.pinned("build.teq", "teq", "1.2.3", Export.Classifiers, repositories, unserved, Logger.Null)
            assertEquals((pins.refusals, pins.untold), (Nil, Nil))
            assertEquals(pins.found, Seq(
              Export.Binary("osx-aarch_64", s"$second$at/teq-1.2.3-osx-aarch_64.exe", Sha1.compute(new File(releases, s"$at/teq-1.2.3-osx-aarch_64.exe")), 12, Some(repository("second", second))),
              Export.Binary("linux-x86_64", s"$second$at/teq-1.2.3-linux-x86_64.exe", Sha1.compute(new File(releases, s"$at/teq-1.2.3-linux-x86_64.exe")), 12, Some(repository("second", second))),
            ))
            assertEquals(Lock.canonical(Export.Binaries("1.2.3", pins.found, Nil).block),
              pins.found.sortBy(_.classifier).map(b => s"${b.classifier}: $second$at/teq-1.2.3-${b.classifier}.exe ${b.sha1} 12\n").mkString)
            // The first repository asked for the pom alone; the second for the pom, then per classifier
            // the .sha1 and a HEAD of the binary (which tells a classifier not published from a binary
            // without its checksum): no binary's GET; the third never asked.
            assertEquals(firstRequests(), Seq(s"HEAD /$at/teq-1.2.3.pom"))
            assertEquals(requests(), s"HEAD /$at/teq-1.2.3.pom" +: Export.Classifiers.flatMap(c => Seq(s"GET /$at/teq-1.2.3-$c.exe.sha1", s"HEAD /$at/teq-1.2.3-$c.exe")))
            assertEquals(thirdRequests(), Nil)
          }
        }
      }
    }
  }

  test("a repository refusing a HEAD, of its own or of a signed URL it redirects to: the pom's presence and each binary's size from a GET of its first byte, a Content-Range's total or an ignored range's Content-Length, the URL the repository's") {
    withDirectory { root =>
      val at = "build/teq/teq/1.2.3"
      published(root, "1.2.3", "linux-x86_64" -> "linux binary", "osx-aarch_64" -> "a mac binary!")
      // The binaries at signed URLs too, as a repository that redirects its downloads serves them.
      for (c <- Seq("linux-x86_64", "osx-aarch_64")) IO.copyFile(new File(root, s"$at/teq-1.2.3-$c.exe"), new File(root, s"signed/teq-1.2.3-$c.exe"))
      def pinned(url: String) = Export.pinned("build.teq", "teq", "1.2.3", Export.Classifiers, Seq(repository("r", url)), unserved, Logger.Null)
      def found(url: String) = Seq("osx-aarch_64" -> 13, "linux-x86_64" -> 12).map { case (c, size) =>
        Export.Binary(c, s"$url$at/teq-1.2.3-$c.exe", Sha1.compute(new File(root, s"$at/teq-1.2.3-$c.exe")), size, Some(repository("r", url)))
      }
      val first = "(bytes=0-0)"
      // Every HEAD refused (405), a range answered (206) or ignored (200): the pom, then per classifier the .sha1, the
      // HEAD and the GET of the first byte; the classifiers not published answer that GET 404.
      for (ranges <- Seq(true, false))
        withRepository(root, head = _ => Some(405), ranged = ranges) { (url, requests) =>
          val pins = pinned(url)
          assertEquals((pins.found, pins.refusals, pins.untold), (found(url), Nil, Nil), s"ranges answered: $ranges")
          assertEquals(requests(), Seq(s"HEAD /$at/teq-1.2.3.pom", s"GET /$at/teq-1.2.3.pom $first") ++
            Export.Classifiers.flatMap(c => Seq(s"GET /$at/teq-1.2.3-$c.exe.sha1", s"HEAD /$at/teq-1.2.3-$c.exe", s"GET /$at/teq-1.2.3-$c.exe $first")))
        }
      // The binaries redirected to signed URLs, which refuse a HEAD (403): the GET of the first byte follows the
      // redirect from the repository's URL, which the record keeps.
      val signed = (path: String) => Option.when(path.startsWith(s"/$at/") && path.endsWith(".exe") && new File(root, s"signed/${path.split('/').last}").isFile)(Right(s"/signed/${path.split('/').last}"))
      withRepository(root, answer = signed, head = path => Option.when(path.startsWith("/signed/"))(403), ranged = true) { (url, requests) =>
        val pins = pinned(url)
        assertEquals((pins.found, pins.refusals, pins.untold), (found(url), Nil, Nil))
        assert(requests().containsSlice(Seq(s"HEAD /$at/teq-1.2.3-linux-x86_64.exe", "HEAD /signed/teq-1.2.3-linux-x86_64.exe",
          s"GET /$at/teq-1.2.3-linux-x86_64.exe $first", s"GET /signed/teq-1.2.3-linux-x86_64.exe $first")), requests())
      }
      // A HEAD refused and the GET refused as well: the repository cannot tell, named by the GET.
      withRepository(root, head = _ => Some(405), answer = path => Option.when(path.endsWith(".pom"))(Left(403))) { (url, _) =>
        assertEquals(pinned(url), Export.Pinned(Nil, Nil, Seq(s"whether the repository r ($url) serves build.teq:teq:1.2.3 cannot be told (GET (bytes=0-0) $url$at/teq-1.2.3.pom answered 403)")))
      }
    }
  }

  test("the binaries' records: a .sha1 that holds no SHA-1, a binary without its .sha1, a repository that answers 500, stalls or redirects nowhere, refused naming it") {
    withDirectory { root =>
      published(root, "1.2.3", "linux-x86_64" -> "linux binary", "osx-aarch_64" -> "a mac binary")
      val at = "build/teq/teq/1.2.3"
      IO.write(new File(root, s"$at/teq-1.2.3-osx-aarch_64.exe.sha1"), "e5f121495fbad0018189acfd347a37de61d3edf\n")
      def pinned(repositories: Export.Repository*) = Export.pinned("build.teq", "teq", "1.2.3", Export.Classifiers, repositories, unserved, Logger.Null)
      def refusal(url: String, classifier: String, why: String) = s"teq's binary for $classifier cannot be pinned from the repository r ($url): $why"
      def osx(url: String) = refusal(url, "osx-aarch_64", s"$url$at/teq-1.2.3-osx-aarch_64.exe.sha1 holds no SHA-1: " + "\"e5f121495fbad0018189acfd347a37de61d3edf\"")
      withRepository(root) { case (url, _) =>
        val pins = pinned(repository("r", url))
        assertEquals((pins.found.map(_.classifier), pins.refusals, pins.untold), (Seq("linux-x86_64"), Seq(osx(url)), Nil))
      }
      // A classifier's .sha1 or binary answering 500, the binary served without its .sha1 (404), and
      // the binary of a classifier whose .sha1 is 404 answering 500: that classifier refused.
      val (linux, windows) = (s"$at/teq-1.2.3-linux-x86_64.exe", s"$at/teq-1.2.3-windows-x86_64.exe")
      for ((broken, answer, why) <- Seq(
        (s"/$linux.sha1", 500, (url: String) => refusal(url, "linux-x86_64", s"GET $url$linux.sha1 answered 500")),
        (s"/$linux", 500, (url: String) => refusal(url, "linux-x86_64", s"HEAD $url$linux answered 500")),
        (s"/$linux.sha1", 404, (url: String) => refusal(url, "linux-x86_64", s"$url$linux is served without its checksum (GET $url$linux.sha1 answered 404)")),
        (s"/$windows", 500, (url: String) => refusal(url, "windows-x86_64", s"HEAD $url$windows answered 500")),
      ))
        withRepository(root, path => Option.when(path == broken)(Left(answer))) { case (url, _) =>
          val pinnedLinux = Option.when(broken.contains("windows"))("linux-x86_64").toSeq
          val pins = pinned(repository("r", url))
          assertEquals((pins.found.map(_.classifier), pins.refusals), (pinnedLinux, Seq(osx(url), why(url))), broken)
        }
      // A .sha1 whose body stalls past the deadline, and one redirected to a location that is no URI.
      def refusalOf(url: String) = refusal(url, "linux-x86_64", s"GET $url$linux.sha1: ")
      withRepository(root, stalled = _ == s"/$linux.sha1") { case (url, _) =>
        val started = System.nanoTime
        val pins = Export.pinned("build.teq", "teq", "1.2.3", Seq("linux-x86_64"), Seq(repository("r", url)), Served(Nil, Logger.Null, Duration.ofSeconds(1)), Logger.Null)
        assertEquals(pins, Export.Pinned(Nil, Seq(refusalOf(url) + "HttpTimeoutException: no whole answer within 1000 ms"), Nil))
        assert(System.nanoTime - started < TimeUnit.SECONDS.toNanos(10))
      }
      withRepository(root, path => Option.when(path == s"/$linux.sha1")(Right("/bad redirect"))) { case (url, _) =>
        val pins = pinned(repository("r", url))
        assertEquals(pins.found, Nil)
        assert(pins.refusals.exists(_.startsWith(refusalOf(url) + "IllegalArgumentException: Illegal character in path")), pins.refusals)
      }
      // The repository answering 500 for the pom, and one out of reach: whether they serve the version
      // cannot be told, no record and no refusal when none serves it (the export warns and names no
      // binary), passed over with a warning when a later one does.
      val closed = { val s = new java.net.ServerSocket(0, 0, InetAddress.getLoopbackAddress); try s.getLocalPort finally s.close() }
      val unreachable = repository("gone", s"http://127.0.0.1:$closed/")
      withRepository(root, path => Option.when(path.endsWith(".pom"))(Left(500))) { case (failing, _) =>
        val pins = pinned(repository("failing", failing), unreachable)
        assertEquals((pins.found, pins.refusals, pins.untold.size), (Nil, Nil, 2))
        assertEquals(pins.untold(0), s"whether the repository failing ($failing) serves build.teq:teq:1.2.3 cannot be told (HEAD $failing$at/teq-1.2.3.pom answered 500)")
        assert(pins.untold(1).startsWith(s"whether the repository gone (http://127.0.0.1:$closed/) serves build.teq:teq:1.2.3 cannot be told (HEAD http://127.0.0.1:$closed/$at/teq-1.2.3.pom: ConnectException"), pins.untold(1))
        withRepository(root) { case (serving, _) =>
          val log = new Recorded()
          val pins = Export.pinned("build.teq", "teq", "1.2.3", Seq("linux-x86_64"), Seq(repository("failing", failing), repository("serving", serving)), unserved, log)
          assertEquals((pins.found.flatMap(_.repository).map(_.id), pins.refusals, pins.untold), (Seq("serving"), Nil, Nil))
          assertEquals(log.lines.toSeq, Seq(s"warn: teq: whether the repository failing ($failing) serves build.teq:teq:1.2.3 cannot be told (HEAD $failing$at/teq-1.2.3.pom answered 500); serving serves it"))
        }
      }
    }
  }

  test("the binaries' records from a repository behind basic authentication, with sbt's credentials for its host, through a redirect, the resolver's URL pinned") {
    withDirectory { root =>
      published(new File(root, "releases"), "1.2.3", "linux-x86_64" -> "linux binary")
      val file = new File(root, "elsewhere.credentials")
      IO.write(file, "realm=teq\nhost=127.0.0.1\nuser=reader\npassword=secret\n")
      val at = "build/teq/teq/1.2.3"
      withRepository(root, path => Option.when(path.startsWith("/moved/"))(Right(path.replace("/moved/", "/releases/"))), user = Some("reader" -> "secret")) { case (url, requests) =>
        val moved = repository("private", s"${url}moved/")
        def pinned(credentials: Seq[Credentials]) = Export.pinned("build.teq", "teq", "1.2.3", Seq("linux-x86_64"), Seq(moved), Served(credentials, Logger.Null), Logger.Null)
        // A credentials file of the build's (sbt's `Credentials(file)`, at a path of its own) and inline
        // credentials alike; every request redirected, the record keeping the URL of the resolver's
        // layout, not the one the redirect answered from.
        for (credentials <- Seq(Seq(Credentials(file)), Seq(Credentials("teq", "127.0.0.1", "reader", "secret")))) {
          val pins = pinned(credentials)
          assertEquals((pins.refusals, pins.untold), (Nil, Nil))
          assertEquals(pins.found.map(b => (b.url, b.sha1, b.size)), Seq((s"${url}moved/$at/teq-1.2.3-linux-x86_64.exe", Sha1.compute(new File(root, s"releases/$at/teq-1.2.3-linux-x86_64.exe")), 12L)))
        }
        assertEquals(requests().filter(_.contains("/releases/")).distinct, Seq(s"HEAD /releases/$at/teq-1.2.3.pom", s"GET /releases/$at/teq-1.2.3-linux-x86_64.exe.sha1", s"HEAD /releases/$at/teq-1.2.3-linux-x86_64.exe"))
        // Without them, or with another host's: the repository cannot tell, naming the answer.
        for (credentials <- Seq(Nil, Seq(Credentials("teq", "elsewhere.example", "reader", "secret"))))
          assertEquals(pinned(credentials), Export.Pinned(Nil, Nil, Seq(s"whether the repository private (${url}moved/) serves build.teq:teq:1.2.3 cannot be told (HEAD ${url}moved/$at/teq-1.2.3.pom answered 401)")))
        // A credentials file that cannot be read is passed over with a warning, as sbt passes it over.
        val log = new Recorded()
        Served(Seq(Credentials(new File(root, "missing"))), log)
        assert(log.lines.exists(_.startsWith("warn: teq: Credentials file")), log.lines)
      }
    }
  }

  test("the cache root, by the rule of teq's jar cache") {
    def root(env: Map[String, String], os: String) = Export.cacheRoot(env.get, os).map(_.getPath)
    val home = Map("HOME" -> "/h")
    assertEquals(root(home + ("TEQ_CACHE_DIR" -> "/t") + ("XDG_CACHE_HOME" -> "/x"), "Mac OS X"), Some("/t"))
    assertEquals(root(home + ("TEQ_CACHE_DIR" -> "") + ("XDG_CACHE_HOME" -> "/x"), "Linux"), Some(new File("/x/teq").getPath))
    assertEquals(root(home, "Mac OS X"), Some(new File("/h/Library/Caches/teq").getPath))
    assertEquals(root(home, "Linux"), Some(new File("/h/.cache/teq").getPath))
    assertEquals(root(Map("LOCALAPPDATA" -> "C:\\Users\\u\\AppData\\Local"), "Windows 11"), Some(new File("C:\\Users\\u\\AppData\\Local", "teq").getPath))
    assertEquals(root(Map.empty, "Linux"), None)
  }

  test("a generator's program found by PATHEXT on Windows") {
    withDirectory { dir =>
      IO.write(new File(dir, "npm.cmd"), "")
      IO.write(new File(dir, "node.exe"), "")
      val env = Map("PATH" -> s"${new File(dir, "none").getPath};${dir.getPath}", "PATHEXT" -> ".COM;.EXE;.BAT;.CMD")
      def program(name: String, os: String = "Windows 11") = Export.program(name, env.get, os)
      assertEquals(program("npm"), new File(dir, "npm.cmd").getPath)
      assertEquals(program("node"), new File(dir, "node.exe").getPath)
      assertEquals(program("npm", "Linux"), "npm")
      assertEquals(program("yarn"), "yarn")
      assertEquals(program("npm.cmd"), "npm.cmd")
      assertEquals(program("scripts/gen.sh"), "scripts/gen.sh")
    }
  }

  test("the resolved binary shared by its digest") {
    withDirectory { dir =>
      val binary = new File(dir, "teq-0.1.0-pre.2-osx-aarch_64")
      IO.write(binary, "binary")
      val cache = new File(dir, "cache")
      val shared = new File(cache, "bin/abc/teq-0.1.0-pre.2-osx-aarch_64")
      Export.share(binary, "abc", binary.getName, Some(cache), Logger.Null)
      assertEquals(IO.read(shared), "binary")
      assert(shared.canExecute)
      IO.write(shared, "binarx")
      Export.share(binary, "abc", binary.getName, Some(cache), Logger.Null)
      assertEquals(IO.read(shared), "binarx", "a file of the same size stays")
      IO.write(shared, "torn")
      Export.share(binary, "abc", binary.getName, Some(cache), Logger.Null)
      assertEquals(IO.read(shared), "binary")
      assertEquals(Option(shared.getParentFile.list).toSeq.flatten, Seq(binary.getName), "no partial file is left")
    }
  }

  test("a TeqCommand runs from the root with the directory to write into, its sources given back") {
    withDirectory { root =>
      val out = new File(root, "target/gen/teq")
      val command = TeqCommand(Seq("sh", "-c", "mkdir -p \"$0/demo\" && printf 'object A\\n' > \"$0/demo/A.scala\" && pwd > \"$0/cwd.txt\""))
      assertEquals(Export.generate(Seq(command), root, out, None, Logger.Null), Seq(new File(out, "demo/A.scala")))
      assertEquals(IO.read(new File(out, "cwd.txt")).trim, root.getCanonicalPath)
      intercept[sbt.MessageOnlyException](Export.generate(Seq(TeqCommand(Seq("sh", "-c", "exit 3"))), root, out, None, Logger.Null))
    }
  }

  test("a TeqCommand whose first word is teq runs the build's binary, given by its path, never a teq of the PATH") {
    assume(!System.getProperty("os.name").toLowerCase.startsWith("windows"), "the stand-in binary is a POSIX shell's script")
    withDirectory { root =>
      val out = new File(root, "target/gen/teq")
      val binary = new File(root, "bin dir/teq-binary")
      IO.write(binary, "#!/bin/sh\nmkdir -p \"$5/demo\" && printf 'object B\\n' > \"$5/demo/B.scala\" && echo \"$1 $2 $4\" > \"$5/args.txt\"\n")
      binary.setExecutable(true)
      IO.write(new File(root, "gen.scala"), "")
      val command = TeqCommand(Seq("teq", "interp", "gen.scala", "--", "app"), outputs = Seq("app/assets/module.js"))
      assert(command.runsTeq)
      assertEquals(Export.generate(Seq(command), root, out, Some(binary), Logger.Null), Seq(new File(out, "demo/B.scala")))
      assertEquals(IO.read(new File(out, "args.txt")).trim, "interp gen.scala app")
      intercept[sbt.MessageOnlyException](Export.generate(Seq(command), root, out, None, Logger.Null))
    }
  }

  test("a TeqCommand's block: the directory appended to run, its outputs that directory and its own") {
    withDirectory { dir =>
      val root = dir.getCanonicalFile.toPath
      val refusals = mutable.ArrayBuffer.empty[String]
      val command = TeqCommand(Seq("teq", "interp", "scripts/gen.scala", "--", "app"), inputs = Seq("app/assets/**/*.svg"), outputs = Seq("app/assets/module.js", "./app/gen/"))
      val block = Export.commandJson(root, command, "target/teq/app/compile/src_managed", "app", refusals, buildTool = true)
      assertEquals(block("run").strings, Seq("teq", "interp", "scripts/gen.scala", "--", "app", "target/teq/app/compile/src_managed"))
      assertEquals(block("outputs").strings, Seq("target/teq/app/compile/src_managed", "app/assets/module.js", "app/gen"))
      assertEquals(block("inputs").strings, Seq("app/assets/**/*.svg"))
      assert(refusals.isEmpty)
      Export.commandJson(root, command.copy(outputs = Seq("../elsewhere.js")), "target/teq/app/compile/src_managed", "app", refusals, buildTool = true)
      assertEquals(refusals.toSeq, Seq("app: the TeqCommand teq interp scripts/gen.scala -- app writes ../elsewhere.js, outside the build's root"))
    }
  }

  test("a TeqCommand's inputs for sbt's watch: its globs and the files its arguments name") {
    withDirectory { root =>
      IO.write(new File(root, "gen.mjs"), "")
      IO.write(new File(root, "labels.txt"), "")
      val globs = Export.generatorInputs(Seq(TeqCommand(Seq("node", "gen.mjs", "missing.txt"), inputs = Seq("assets/**/*.svg"))), root)
      val base = root.getCanonicalFile.toPath
      assert(globs.exists(_.matches(base.resolve("assets/a/b.svg"))))
      assert(globs.exists(_.matches(new File(root, "gen.mjs").getCanonicalFile.toPath)))
      assert(!globs.exists(_.matches(new File(root, "labels.txt").getCanonicalFile.toPath)))
    }
  }

  test("a jar's name in the Docker stage, as native-packager's makeJarName gives it") {
    assertEquals(Export.jarName("org.typelevel", "cats-core_3", "2.13.0", "cats-core_3", None), "org.typelevel.cats-core_3-2.13.0.jar")
    assertEquals(Export.jarName("api", "api", "0.1.0-SNAPSHOT", "api", Some("")), "api.api-0.1.0-SNAPSHOT.jar")
    assertEquals(Export.jarName("io.netty", "netty-transport-native-epoll", "4.1.0", "netty-transport-native-epoll", Some("linux-x86_64")), "io.netty.netty-transport-native-epoll-4.1.0-linux-x86_64.jar")
    assertEquals(Export.jarName("com.example", "core", "1.0", "core-extras", None), "com.example.core--extras-1.0.jar")
  }

  test("the launchers: written when absent, an unedited one of any version replaced, any other kept and reported") {
    val v1 = Map("teq" -> "#!/bin/sh\n# teq launcher 1: the first\nexec one\n", "teq.cmd" -> "@echo off\nrem teq launcher 1: the first\none\n")
    val v2 = Map("teq" -> "#!/bin/sh\n# teq launcher 2: the second\nexec two\n", "teq.cmd" -> "@echo off\nrem teq launcher 2: the second\ntwo\n")
    val templates = Seq(1 -> v1, 2 -> v2)
    withDirectory { root =>
      val (sh, cmd) = (new File(root, "teq"), new File(root, "teq.cmd"))
      def write() = Launchers.write(root, templates).map(r => (r.kept, r.message.replace(root.getPath + File.separator, "")))
      // Absent: written, the sh one executable, the cmd one with cmd's line ends.
      assertEquals(write(), Seq(false -> "wrote teq, launcher 2", false -> "wrote teq.cmd, launcher 2"))
      assertEquals((IO.read(sh), IO.read(cmd)), (v2("teq"), v2("teq.cmd").replace("\n", "\r\n")))
      assert(sh.canExecute)
      // Current: left alone, its execution bit set again.
      sh.setExecutable(false)
      assertEquals(write(), Nil)
      assert(sh.canExecute)
      // An unedited launcher 1, the cmd one checked out with \n alone: replaced.
      IO.write(sh, v1("teq"))
      IO.write(cmd, v1("teq.cmd"))
      assertEquals(write(), Seq(false -> "replaced teq, launcher 1, by launcher 2", false -> "replaced teq.cmd, launcher 1, by launcher 2"))
      // The current one checked out with \r\n: current.
      IO.write(sh, v2("teq").replace("\n", "\r\n"))
      assertEquals(write(), Nil)
      // An edited launcher 1, one without the marker, one of a later plugin: kept, each said why, the update's first difference named.
      IO.write(sh, v1("teq").replace("exec one", "export X=1\nexec one"))
      IO.write(cmd, "@echo off\nmine\n")
      assertEquals(write(), Seq(
        true -> "kept teq, since it is launcher 1, edited: launcher 2 differs from it first at line 2, `# teq launcher 2: the second`",
        true -> "kept teq.cmd, since it has no `teq launcher <version>:` line: launcher 2 differs from it first at line 2, `rem teq launcher 2: the second`",
      ))
      assertEquals(IO.read(cmd), "@echo off\nmine\n")
      IO.write(sh, "#!/bin/sh\n# teq launcher 3: the third\n")
      assertEquals(write().head, true -> "kept teq, since it is launcher 3, a later sbt-teq's: launcher 2 differs from it first at line 2, `# teq launcher 2: the second`")
    }
  }

  test("the launchers the plugin carries: teq's tools/launcher, the current one last") {
    val (version, current) = Launchers.shipped.last
    for (name <- Launchers.Names) assert(current(name).contains(s"teq launcher $version:"), name)
    assert(!current("teq.cmd").contains("\r"), "carried with \\n line ends")
  }

  test("the lock's place: the build's root under teqBuildTool, else its target/teq/, where every reader looks after the root") {
    val root = new File("/b")
    assertEquals(Export.location(root, buildTool = true), new File("/b/teq.lock"))
    assertEquals(Export.location(root, buildTool = false), new File("/b/target/teq/teq.lock"))
  }

  test("a description's managed source directories: the export's directory of a project whose generators teq runs, sbt's own of one whose generators sbt runs or that has no part, none of one without generators") {
    def part(name: String, sources: Seq[Either[String, Export.Managed]], bySbt: Option[Boolean]) =
      Export.Part(name, Json.Obj(Map("base" -> Json.Str(name), "description" -> Json.Obj(Map()))), Nil, Nil, Nil, Nil, sources, bySbt)
    def managed(owner: String, dir: String, generates: Boolean = true) = Right(Export.Managed(owner, Some(dir), generates))
    val app = part("app", Seq(
      Left("app/src"),
      managed("gen", "target/out/gen/src_managed/main"),
      managed("multi", "multi/managed-a"),
      managed("teq", "target/out/teq/src_managed/main"), managed("teq", "target/out/teq/src_managed/other"),
      managed("plain", "target/out/plain/src_managed/main", generates = false),
      managed("foreign", "target/out/foreign/src_managed/main"),
      managed("idle", "target/out/idle/src_managed/main", generates = false),
      managed("foreign", "/elsewhere/foreign/src_managed/main"),
      Right(Export.Managed("gen", None, true)),
    ), None)
    val parts = Seq(app, part("gen", Nil, Some(true)), part("multi", Nil, Some(true)), part("teq", Nil, Some(false)), part("plain", Nil, None))
    val sources = Export.withSources(parts).map(p => p.name -> p.json("description")("sources").strings).toMap
    assertEquals(sources("app"), Seq("app/src", "target/out/gen/src_managed/main", "multi/managed-a", "target/teq/teq/compile/src_managed", "target/out/foreign/src_managed/main", "/elsewhere/foreign/src_managed/main"))
    assertEquals(sources("gen"), Nil)
  }
}
