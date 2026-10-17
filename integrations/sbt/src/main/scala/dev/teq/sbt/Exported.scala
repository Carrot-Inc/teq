package dev.teq.sbt

import java.io.File
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path}
import java.security.MessageDigest
import scala.collection.mutable
import scala.util.control.NonFatal
import scala.collection.compat.*

/** A source generator that is a program the repository holds: run from `cwd`, a directory under
  * the build's root, with `run`'s arguments and the directory to write into appended, whenever
  * the build tool's compile asks for the configuration's sources (sbt's `Compile / teqGenerators`,
  * mill-teq's overlay). The first word `teq` is the build's own binary (`teqResolvedBinary`; under
  * `teq`, the one running), so that `teq interp` runs a script with no other tool. `inputs` are globs
  * under the build's root whose change has a watch run it again; `teq` runs it again when they or the
  * files its arguments name change, or when one of `outputs` is missing: files or directories under
  * the build's root the program writes besides the directory appended (a module of the application's
  * assets, say). */
final case class TeqCommand(run: Seq[String], inputs: Seq[String] = Nil, cwd: String = ".", outputs: Seq[String] = Nil) {
  /** Whether the build's own binary runs it. */
  def runsTeq: Boolean = run.headOption.contains("teq")
}

/** What every exporter of teq.lock writes alike (docs/TOOLING.md, "The lockfile and the launchers"):
  * sbt-teq's `Export` and mill-teq's `dev.teq.mill.Export`, from their own build's facts. The records
  * of the lock (repositories, jars, binaries, a project's part) and the document they make, the paths
  * as the lock names them, the keys and layouts of the jar table, the command generators and the dev
  * block, and the binaries a compiler's release or a repository serves. */
private[teq] object Exported {
  val FileName = Lock.FileName

  /** The build's inputs as the lock records them: each file with its SHA-256, the SHA-256 of
    * `<path>\0<digest>\n` over them, and the inputs the build declares besides its definition (Mill's). */
  final case class Inputs(files: Seq[(String, String)], sha256: String, declared: Seq[String] = Nil)

  /** The inputs block of these files (by path relative to the root with `/`, in the order of the paths'
    * bytes): those that read, with the digests of their contents. */
  def digests(files: Seq[(String, Path)], declared: Seq[String] = Nil): Inputs = {
    val read = files.flatMap { case (path, p) =>
      try Some(path -> hex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(p))))
      catch { case NonFatal(_) => None }
    }
    val sha = MessageDigest.getInstance("SHA-256")
    for ((path, digest) <- read) sha.update(s"${path}\u0000${digest}\n".getBytes(UTF_8))
    Inputs(read, hex(sha.digest()), declared)
  }

  val Classifiers = Seq("osx-aarch_64", "osx-x86_64", "linux-x86_64", "linux-aarch_64", "windows-x86_64")

  val MavenCentral = "maven-central"
  val MavenCentralRoot = "https://repo1.maven.org/maven2/"
  /** The class files' version teq writes where no option names one. */
  val DefaultJavaOutputVersion = 17

  final case class Repository(id: String, url: String, credentials: Option[String]) {
    def json: Json.Value = Json.Obj(Map("id" -> Json.Str(id), "url" -> Json.Str(url)) ++ credentials.map(host => "credentials" -> Json.Str(host)))
  }

  /** A record of the jar table under its key, and the configuration whose classpath named it; its
    * repository by its URL and credentials host, whatever the project named it. */
  final case class Jar(key: String, repository: Repository, path: String, sha1: String, size: Long, where: String) {
    def record: (String, Option[String], String, String, Long) = (repository.url, repository.credentials, path, sha1, size)
    /** Its record in the lock, its fields: the repository's id, the sha1, the size, and the path
      * where it is not the Maven layout of the key. */
    def fields(id: Repository => String): Json.Value =
      Json.Str((Seq(id(repository), sha1, size.toString) ++ Option.when(mavenLayout(key).forall(_ != path))(path)).mkString(" "))
    override def toString = s"${repository.url}$path (sha1 $sha1, $size bytes) for $where"
  }

  /** A project's part of the lock: `sources`, its description's source roots, a managed source
    * directory of a project by that project (`withSources` writes them); `bySbt`, whether its
    * compile generators are sbt's (none without generators); `skipped`, why it is left out (a
    * platform no reader takes). */
  final case class Part(name: String, json: Json.Value, repositories: Seq[Repository], jars: Seq[Jar], javaOutputVersions: Seq[Int], refusals: Seq[String], sources: Seq[Either[String, Managed]] = Nil, bySbt: Option[Boolean] = None, skipped: Option[String] = None)

  /** A project's managed source directory of Compile among a description's source roots: the
    * project's name, the directory as the lock names it (relative to the build's root; outside
    * it absolute without the driver, none under it), and whether the project has generators.
    * What it stands for is known once every project's part says whose its generators are. */
  final case class Managed(owner: String, path: Option[String], generates: Boolean)

  /** A binary of the compiler for a classifier: its URL and what the export pins of it, and the Maven repository it
    * resolves from for a SNAPSHOT published locally (none for a release's asset). */
  final case class Binary(classifier: String, url: String, sha1: String, size: Long, repository: Option[Repository] = None)

  final case class Binaries(version: String, found: Seq[Binary], refusals: Seq[String]) {
    def repositories: Seq[Repository] = found.flatMap(_.repository).distinct
    /** The lock's `binaries`: per classifier the binary's fields, its URL, sha1 and size. */
    def block: Json.Value = Json.Obj(found.map(b => b.classifier -> Json.Str(s"${b.url} ${b.sha1} ${b.size}")).toMap)
  }

  /** One repository for each URL and credentials host the projects' resolvers (and the binaries')
    * reach, by the id the first to name it gives it, the projects in the order of their names: two
    * projects' resolvers may name one repository differently. With the repositories in that order,
    * the id each declared one is written with, and a refusal for an id that names two URLs. */
  def shared(declared: Seq[Repository]): (Seq[Repository], Repository => String, Seq[String]) = {
    val first = declared.groupBy(r => (r.url, r.credentials)).map{ case (identity, named) => identity -> named.head}
    val repositories = declared.map(r => first((r.url, r.credentials))).distinct
    val conflicts = repositories.groupBy(_.id).toSeq.sortBy(_._1)(using utf8).collect {
      case (id, named) if named.size > 1 => s"the repository id $id names ${named.map(_.url).mkString(" and ")}: give the resolvers names of their own"
    }
    (repositories, r => first((r.url, r.credentials)).id, conflicts)
  }

  /** The jar table, one record per key, and a refusal for every key that names two jars (the
    * same coordinates from two repositories, at two paths, or with two digests or sizes) and for
    * a path with a space, which the record's fields cannot hold. */
  def table(jars: Seq[Jar]): (Seq[Jar], Seq[String]) = {
    val byKey = jars.groupBy(_.key).toSeq.sortBy(_._1)(using utf8)
    val refusals = byKey.flatMap { case (key, named) =>
      named.distinctBy(_.record) match {
        case Seq(_) => None
        case first +: second +: _ => Some(s"the jar $key is two files, $first and $second: one key names one jar")
      }
    } ++ byKey.map(_._2.head).filter(_.path.contains(' ')).map(j => s"the jar ${j.key} is at $j, a path with a space, which $FileName's fields cannot hold")
    (byKey.map(_._2.head), refusals)
  }

  def document(inputs: Inputs, parts: Seq[Part], repositories: Seq[Repository], id: Repository => String, jars: Seq[Jar], binaries: Binaries, extra: Map[String, Json.Value] = Map.empty): Json.Value = {
    val outputVersion = (parts.flatMap(_.javaOutputVersions) :+ DefaultJavaOutputVersion).max
    val (inputFiles, inputHash) = (inputs.files, inputs.sha256)
    Json.Obj(Map(
      "teq" -> Json.Str(binaries.version),
      "format" -> num(Lock.Format),
      "binaries" -> binaries.block,
      "inputs" -> Json.Obj(Map("files" -> Json.Obj(inputFiles.map{ case (f, d) => f -> Json.Str(d)}.toMap), "sha256" -> Json.Str(inputHash)) ++
        Option.when(inputs.declared.nonEmpty)("declared" -> strings(inputs.declared))),
      "repositories" -> Json.Arr(repositories.map(_.json)),
      "jars" -> Json.Obj(jars.map(j => j.key -> j.fields(id)).toMap),
      "java" -> Json.Obj(Map("outputVersion" -> num(outputVersion))),
      "projects" -> Json.Obj(parts.map(p => p.name -> p.json).toMap),
    ) ++ extra)
  }

  def hex(bytes: Array[Byte]): String = bytes.map(b => f"${b & 0xff}%02x").mkString

  val utf8: Ordering[String] = Json.utf8

  def num(n: Long): Json.Value = Json.Num(n.toString)
  def strings(items: Seq[String]): Json.Value = Json.Arr(items.map(Json.Str(_)))

  def rootOf(base: File): Path = base.toPath.toAbsolutePath.normalize

  /** A path under the build's root, relative with `/`; `.` for the root itself. */
  def relativeTo(root: Path, file: File): String = {
    val path = file.toPath.toAbsolutePath.normalize
    root.relativize(path).toString.replace('\\', '/') match {
      case "" => "."
      case relative => relative
    }
  }

  def under(root: Path, file: File): Boolean = file.toPath.toAbsolutePath.normalize.startsWith(root)

  /** A path outside the build's root as a lock without the driver names it: absolute, with `/`. */
  def absolute(file: File): String = file.toPath.toAbsolutePath.normalize.toString.replace('\\', '/')

  def generatedDirectory(project: String, configuration: String = "compile"): String = s"target/teq/$project/$configuration/src_managed"

  /** A jar's key in the table: `organization:name:version`, and `:classifier` when it has one. */
  def jarKey(organization: String, name: String, version: String, classifier: Option[String]): String =
    (Seq(organization, name, version) ++ classifier.filter(_.nonEmpty)).mkString(":")

  /** The Maven layout of a jar's key, the path a reader derives where the table names none:
    * `org/with/slashes/name/version/name-version[-classifier].jar`; none for a string that is no
    * `organization:name:version[:classifier]` (`task::export::maven_path` in teq). */
  def mavenLayout(key: String): Option[String] =
    key.split(":", -1).toSeq match {
      case parts if (parts.size == 3 || parts.size == 4) && parts.forall(_.nonEmpty) =>
        val Seq(organization, name, version) = parts.take(3)
        Some(s"${organization.replace('.', '/')}/$name/$version/$name-$version${parts.drop(3).map("-" + _).mkString}.jar")
      case _ => None
    }

  def isSnapshot(version: String): Boolean = version.endsWith("-SNAPSHOT")

  def isDynamic(revision: String): Boolean =
    revision.endsWith("+") || revision.startsWith("latest.") || revision.exists("[]()".contains(_))

  /** The repository a URL lies under: the one of the longest root. */
  def repositoryOf(repositories: Seq[Repository], url: String): Option[Repository] =
    repositories.filter(r => url.startsWith(r.url)).maxByOption(_.url.length)

  def slug(name: String): String =
    name.toLowerCase.replaceAll("[^a-z0-9]+", "-").stripPrefix("-").stripSuffix("-") match {
      case "" => "repository"
      case s => s
    }

  /** The URL of a file coursier's cache holds (`<cache>/https/<host>/<path>`, a character
    * coursier's `CachePath.escape` takes as `%` and two digits of base 16, such as a port's `:`,
    * read back). */
  def cachedUrl(cache: File, file: File): Option[String] = {
    val path = file.toPath.toAbsolutePath.normalize
    val root = cache.toPath.toAbsolutePath.normalize
    Option.when(path.startsWith(root))(root.relativize(path).toString.replace('\\', '/')).collect {
      case relative if relative.startsWith("https/") || relative.startsWith("http/") =>
        val (scheme, rest) = relative.span(_ != '/')
        val unescaped = "%([0-9A-F]{2})".r.replaceAllIn(rest, (m: scala.util.matching.Regex.Match) => scala.util.matching.Regex.quoteReplacement(Integer.parseInt(m.group(1), 16).toChar.toString))
        s"$scheme:/$unescaped"
    }
  }

  /** A command generator's block: the command with the generated directory appended, relative to
    * its working directory; its outputs the generated directory and the command's own, relative to
    * the build's root. */
  def commandJson(root: Path, command: TeqCommand, output: String, project: String, refusals: mutable.Buffer[String], buildTool: Boolean): Json.Value = {
    val cwd = root.resolve(command.cwd).normalize
    if (buildTool && !cwd.startsWith(root)) refusals += s"$project: the TeqCommand ${command.run.mkString(" ")} runs in ${command.cwd}, outside the build's root"
    val out = cwd.relativize(root.resolve(output)).toString.replace('\\', '/')
    val written = command.outputs.map { o =>
      val path = root.resolve(o).normalize
      if (buildTool && !path.startsWith(root)) refusals += s"$project: the TeqCommand ${command.run.mkString(" ")} writes $o, outside the build's root"
      if (path.startsWith(root)) relativeTo(root, path.toFile) else absolute(path.toFile)
    }
    Json.Obj(Map(
      "kind" -> Json.Str("command"),
      "run" -> strings(command.run :+ out),
      "cwd" -> Json.Str(if (cwd.startsWith(root)) relativeTo(root, cwd.toFile) else absolute(cwd.toFile)),
      "inputs" -> strings(command.inputs),
      "outputs" -> strings(output +: written),
    ))
  }

  /** `dev` of a Scala.js project: the package manager and the lockfile of the nearest
    * `package.json` at or above its base, and the command, `teqDevCommand` or that package's
    * `dev` script; none without either. */
  def devOf(root: Path, base: File, command: Seq[String]): Option[Json.Value] = {
    val start = base.toPath.toAbsolutePath.normalize
    val packageDir = Iterator.iterate(start)(_.getParent).takeWhile(d => d != null && d.startsWith(root)).find(d => Files.isRegularFile(d.resolve("package.json")))
    packageDir.flatMap { dir =>
      val manifest = try Json.parse(new String(Files.readAllBytes(dir.resolve("package.json")), UTF_8)) catch { case NonFatal(_) => Json.Null }
      val declared = manifest("packageManager").str.takeWhile(_ != '@')
      val lockfiles = Seq("yarn" -> "yarn.lock", "pnpm" -> "pnpm-lock.yaml", "bun" -> "bun.lock", "npm" -> "package-lock.json")
      val manager = Some(declared).filter(m => lockfiles.exists(_._1 == m))
        .orElse(lockfiles.collectFirst { case (m, lock) if Files.isRegularFile(dir.resolve(lock)) => m })
        .getOrElse("npm")
      val script = Option.when(dir == start && !manifest("scripts")("dev").isNull)(Seq(manager, "run", "dev"))
      Some(command).filter(_.nonEmpty).orElse(script).map { run =>
        Json.Obj(Map(
          "packageManager" -> Json.Str(manager),
          "command" -> strings(run),
          "lockfile" -> Json.Str(relativeTo(root, dir.resolve(lockfiles.find(_._1 == manager).get._2).toFile)),
        ))
      }
    }
  }

  /** What the repositories give of the compiler's binaries: the records and the refusals; and when
    * no repository serves the version while some could not tell (out of reach, or answering other
    * than 200 or 404), why each could not: the export then names no binary, with a warning, so that
    * an export with no repository in reach still writes the lock. */
  final case class Pinned(found: Seq[Binary], refusals: Seq[String], untold: Seq[String])

  /** The path of the compiler's pom under a repository, or of a classifier's binary beside it, in
    * the Maven layout a local publish of a SNAPSHOT gives them: `build/teq/teq/<version>/teq-<version>[-<classifier>.exe|.pom]`. */
  def binaryPath(org: String, name: String, version: String, classifier: Option[String]): String =
    s"${org.replace('.', '/')}/$name/$version/$name-$version${classifier.fold(".pom")(c => s"-$c.exe")}"

  /** The compiler's binaries as the repositories serve them, none downloaded. The repositories are
    * asked in the build's order whether they serve the version, by a HEAD of its pom (`Served.probe`:
    * a GET of its first byte where the repository refuses the HEAD): coursier
    * resolves the module by its pom, so a repository serving the binaries without one is never
    * taken, and the first that serves it gives every classifier, a classifier it lacks not looked
    * for in a later one, as in coursier. A repository that cannot tell is passed over with a warning
    * when a later one serves the version, as coursier passes it over. Per classifier, the sha1 is
    * the `.sha1` beside the binary and the size a HEAD's `Content-Length` (or that GET's total), the
    * URL the resolver's layout gives (redirects followed for the requests alone, never a presigned
    * URL they lead to). A classifier is not published when
    * both its `.sha1` and its binary answer 404; a binary served without its `.sha1`, and any other
    * answer, is a refusal naming the classifier. */
  def pinned(org: String, name: String, version: String, classifiers: Seq[String], repositories: Seq[Repository], served: Served, warn: String => Unit): Pinned = {
    val module = s"$org:$name:$version"
    val pom = binaryPath(org, name, version, None)
    val unanswered = mutable.ArrayBuffer.empty[(Repository, String)]
    val serving = repositories.iterator.map { r =>
      served.probe(r.url + pom) match {
        case Right(answer) if answer.status == 200 => Some(r)
        case Right(answer) if answer.status == 404 => None
        case Right(answer) =>
          unanswered += r -> s"${answer.asked} ${r.url}$pom answered ${answer.status}"
          None
        case Left(why) =>
          unanswered += r -> why
          None
      }
    }.collectFirst { case Some(r) => r }
    def described(r: Repository) = s"the repository ${r.id} (${r.url})"
    serving match {
      case None =>
        Pinned(Nil, Nil, unanswered.toSeq.map{ case (r, why) => s"whether ${described(r)} serves $module cannot be told ($why)"})
      case Some(repository) =>
        for ((r, why) <- unanswered) warn(s"teq: whether ${described(r)} serves $module cannot be told ($why); ${repository.id} serves it")
        val records: Seq[Either[String, Binary]] = classifiers.flatMap { classifier =>
          val path = binaryPath(org, name, version, Some(classifier))
          val url = repository.url + path
          def refused(why: String) = Some(Left(s"teq's binary for $classifier cannot be pinned from ${described(repository)}: $why"))
          served.get(url + ".sha1") match {
            case Left(why) => refused(why)
            case Right(answer) if answer.status == 404 =>
              served.probe(url) match {
                case Left(why) => refused(why)
                case Right(head) if head.status == 404 => None
                case Right(head) if head.status == 200 => refused(s"$url is served without its checksum (GET $url.sha1 answered 404)")
                case Right(head) => refused(s"${head.asked} $url answered ${head.status}")
              }
            case Right(answer) if answer.status != 200 => refused(s"GET $url.sha1 answered ${answer.status}")
            case Right(answer) =>
              Sha1.in(answer.body) match {
                case None => refused(s"$url.sha1 holds no SHA-1: " + "\"" + answer.body.trim.linesIterator.nextOption().getOrElse("").take(80) + "\"")
                case Some(sha1) =>
                  served.probe(url) match {
                    case Left(why) => refused(why)
                    case Right(head) if head.status != 200 => refused(s"${head.asked} $url answered ${head.status}")
                    case Right(head) =>
                      head.length.filter(_ >= 0) match {
                        case None => refused(s"${head.asked} $url gave no size")
                        case Some(size) => Some(Right(Binary(classifier, repository.url.stripSuffix("/") + "/" + path.stripPrefix("/"), sha1, size, Some(repository))))
                      }
                  }
              }
          }
        }
        val (refusals, found) = records.partitionMap(record => record)
        Pinned(found, refusals, Nil)
    }
  }

  /** A configuration's `flags` record, the scalac options as teq honours them (`ScalacOptions`), and the class
    * files' version they name, a JVM configuration's alone. */
  def flagsOf(options: Seq[String], js: Boolean): (Json.Value, Option[Int]) = {
    val o = ScalacOptions(options)
    val fields = Map(
      "strictEquality" -> Json.Bool(o.strictEquality),
      "kindProjector" -> Json.Bool(o.kindProjector),
      "werror" -> Json.Bool(o.werror),
      "ignoredScalacOptions" -> strings(o.ignored),
    ) ++ o.maxInlines.map(n => "maxInlines" -> num(n)) ++ o.javaOutputVersion.map(n => "javaOutputVersion" -> num(n)) ++
      Option.when(o.wunusedImports)("wunusedImports" -> Json.Bool(true)) ++
      Option.when(o.wunused.nonEmpty)("wunused" -> strings(o.wunused)) ++
      Option.when(o.deprecation)("deprecation" -> Json.Bool(true)) ++
      Option.when(o.feature)("feature" -> Json.Bool(true)) ++
      Option.when(o.wtostringInterpolated)("wtostringInterpolated" -> Json.Bool(true)) ++
      Option.when(o.wconf.nonEmpty)("wconf" -> strings(o.wconf)) ++
      Option.when(o.language.nonEmpty)("language" -> strings(o.language)) ++
      o.source.map(v => "source" -> Json.Str(v))
    (Json.Obj(fields), o.javaOutputVersion.filter(_ => !js))
  }

  /** The `binaries` block: the compiler `version` for every classifier its release's manifest lists (under the
    * releases' `base`), or, for a SNAPSHOT published locally, the first of the build's repositories that serves
    * it holds (`pinned`), no binary downloaded. A release before 0.1.7 is refused (`Release.floor`), and so is a
    * release its base does not hold, unless `snapshots` allows the table to stay empty: the table of a release
    * not published yet. `named` are the versions the build names, each by what names it, a SNAPSHOT or dynamic
    * one refused unless `snapshots` (`allow` says what allows it); `warn` is told what leaves the table empty. */
  def binariesOf(org: String, name: String, version: String, base: String, named: Seq[(String, String)], snapshots: Boolean, allow: String,
      repositories: Seq[Repository], served: Served, warn: String => Unit): Binaries = {
    val refusals = mutable.ArrayBuffer.empty[String]
    for ((what, v) <- named if !snapshots && (isSnapshot(v) || isDynamic(v)))
      refusals += s"$what is $v, which a later resolution can make another file: name a release ($allow allows it)"
    if (isSnapshot(version)) {
      val pins = pinned(org, name, version, Classifiers, repositories, served, warn)
      refusals ++= pins.refusals
      if (pins.untold.nonEmpty)
        warn(s"teq: no binary of $org:$name:$version could be read from the build's repositories: ${pins.untold.mkString("; ")}; ${FileName} names none")
      else if (pins.found.isEmpty) {
        val why = s"no binary of $org:$name:$version is served by the build's repositories for any of ${Classifiers.mkString(", ")}"
        if (snapshots) warn(s"teq: $why; ${FileName} names none") else refusals += why
      }
      Binaries(version, pins.found, refusals.toSeq)
    }
    else if (Release.floor(version).nonEmpty) Binaries(version, Nil, (refusals ++ Release.floor(version)).toSeq)
    else
      // A release's binaries, from its manifest, checked against its SHA256SUMS: no binary downloaded.
      Release.manifest(base, version, served) match {
        case Right(m) =>
          if (m.entries.isEmpty) refusals += s"the release v$version's ${Release.manifestName(version)} lists no binary"
          Binaries(version, m.entries.map(e => Binary(e.classifier, Release.assetUrl(base, version, e.classifier), e.sha1, e.size)), refusals.toSeq)
        case Left(unread) if unread.absent =>
          val why = s"no release v$version of teq is at ${Release.directory(base, version)}: ${unread.why}"
          if (snapshots) warn(s"teq: $why; ${FileName} names no binary") else refusals += why
          Binaries(version, Nil, refusals.toSeq)
        case Left(unread) =>
          warn(s"teq: the release v$version of teq could not be read: ${unread.why}; ${FileName} names no binary")
          Binaries(version, Nil, refusals.toSeq)
      }
  }
}
