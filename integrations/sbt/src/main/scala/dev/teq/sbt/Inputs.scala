package dev.teq.sbt

import java.io.File
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.Path

import sbt.*
import sbt.Keys.*

import TeqPlugin.autoImport.*

/** `<project> / <configuration> / teqInputs`: what sbt compiles a configuration from, as concrete
  * paths, for the gate's application lists (bench/app/app-lists.sh; docs/DEVELOPING.md, "The
  * landing gate"). Written as JSON to `target/teq/inputs-<configuration>.json` under the
  * project's base, the file returned:
  *
  *   - `classpath`: the configuration's `dependencyClasspath` in sbt's order, each entry its file
  *     through the build's file converter, and an entry that is a project's products (its class
  *     directory, or the jar `exportJars` packages) named by that project and configuration as well.
  *     A project's products are found by their paths, and an entry whose module or configuration
  *     attribute names another project, or a project's module at a path that is none of its
  *     products, is refused. The configuration's own products are not on it (sbt's
  *     `fullClasspath` adds them, and compiles the configuration to answer); to answer, sbt
  *     compiles and packages the products the entries name.
  *   - `modules`: every project and configuration whose products the classpath holds, upstream
  *     first (a walk of the declared dependencies, each after the ones it depends on), then the
  *     configuration itself: each its source directories as sbt has them (`unmanaged`, `managed`,
  *     existing or not), its `sources` (the generators run), its `scalacOptions` and the teq flags
  *     they map onto, the options teq ignores beside them.
  */
object Inputs:
  @transient val teqInputs = taskKey[File]("Writes what sbt compiles the configuration from as JSON (target/teq/inputs-<configuration>.json): its classpath with each project's products by provenance, and its own and its upstream projects' sources and scalacOptions, for the gate's application lists")

  def projectSettings: Seq[Setting[?]] = Seq(Compile, Test).map(c => c / teqInputs := of(c).value)

  /** A source set and its options: one module of the lists. */
  private final case class Module(project: ProjectRef, configuration: String, unmanaged: Seq[File], managed: Seq[File], sources: Seq[File], options: Seq[String])

  private def module(project: ProjectRef, configuration: String): Def.Initialize[Task[Module]] =
    val c = ConfigKey(configuration)
    Def.task {
      Module(project, configuration, (project / c / unmanagedSourceDirectories).value, (project / c / managedSourceDirectories).value,
        (project / c / sources).value, (project / c / scalacOptions).value)
    }

  private def of(c: Configuration): Def.Initialize[Task[File]] = Def.taskDyn {
    val converter = fileConverter.value
    val data = settingsData.value
    val deps = buildDependencies.value
    val self = thisProjectRef.value
    val projects = loadedBuild.value.allProjectRefs.map(_._1)
    val entries = (c / dependencyClasspath).value
    val out = baseDirectory.value / "target" / "teq" / s"inputs-${c.name}.json"
    def normal(p: Path) = p.toAbsolutePath.normalize
    // Every project's products by their paths, in the configurations a classpath takes them from.
    val products: Map[Path, (ProjectRef, String)] = (for
      p <- projects
      conf <- Seq(Compile, Test)
      path <- (p / conf / classDirectory).get(data).map(_.toPath).toSeq ++ (p / conf / packageBin / artifactPath).get(data).map(converter.toPath).toSeq
    yield normal(path) -> (p, conf.name)).toMap
    def moduleOf(text: String): (String, String, String) =
      val m = Json.parse(text)
      (m("organization").str, m("name").str, m("revision").str)
    val ids: Map[(String, String, String), Seq[ProjectRef]] =
      projects.flatMap(p => (p / projectID).get(data).map(id => (id.organization, id.name, id.revision) -> p)).groupMap(_._1)(_._2)
    /** The configuration named and every one it extends, transitively, by name. */
    def extending(p: ProjectRef, name: String): Set[String] =
      val configs = (p / ivyConfigurations).get(data).getOrElse(Nil).map(c => c.name -> c).toMap
      def up(n: String, seen: Set[String]): Set[String] =
        if seen(n) then seen else configs.get(n).fold(seen + n)(c => c.extendsConfigs.foldLeft(seen + n)((s, e) => up(e.name, s)))
      up(name, Set.empty)
    val classpath = entries.map { entry =>
      val path = normal(converter.toPath(entry.data))
      val named = entry.get(Keys.moduleIDStr).map(moduleOf).flatMap(ids.get).getOrElse(Nil)
      val configuration = entry.get(Keys.configurationStr)
      products.get(path) match
        case Some((p, conf)) =>
          if named.nonEmpty && !named.contains(p) then
            throw new MessageOnlyException(s"teq: ${self.project}/${c.name}: the classpath entry $path is ${p.project}'s $conf products by its path, and its module attribute names ${named.map(_.project).mkString(", ")}")
          // sbt names the configuration a classpath takes the products through: Runtime for Compile's.
          if configuration.exists(named => !extending(p, named).contains(conf)) then
            throw new MessageOnlyException(s"teq: ${self.project}/${c.name}: the classpath entry $path is ${p.project}'s $conf products by its path, and its configuration attribute names ${configuration.get}")
          (path, Some((p, conf)))
        case None if named.nonEmpty =>
          throw new MessageOnlyException(s"teq: ${self.project}/${c.name}: the classpath entry $path names ${named.map(_.project).mkString(", ")}'s module, and is none of its products")
        case None => (path, None)
    }.filterNot(_._2.contains((self, c.name)))
    val upstream = classpath.flatMap(_._2).distinct
    // Upstream first: the declared dependencies walked, each project after the ones it depends on.
    val order = scala.collection.mutable.LinkedHashSet.empty[ProjectRef]
    def walk(p: ProjectRef, seen: Set[ProjectRef]): Unit =
      if !order(p) && !seen(p) then
        for dep <- deps.classpath.getOrElse(p, Nil) do walk(dep.project, seen + p)
        order += p
    walk(self, Set.empty)
    val ranked = upstream.sortBy((p, _) => order.toSeq.indexOf(p) match { case -1 => Int.MaxValue; case i => i })
    val modules = (ranked :+ (self, c.name)).foldLeft(Def.task(Vector.empty[Module]))((all, m) => Def.task(all.value :+ module(m._1, m._2).value))
    Def.task {
      val root = (LocalRootProject / baseDirectory).value.toPath.toAbsolutePath.normalize
      val jvm = TeqPlugin.Jvm == teqTarget.value
      def paths(files: Seq[File]) = Json.Arr(files.map(f => Json.Str(normal(f.toPath).toString)))
      def described(m: Module) =
        val options = TeqPlugin.ScalacOptions(m.options)
        Json.Obj(Map(
          "project" -> Json.Str(m.project.project),
          "configuration" -> Json.Str(m.configuration),
          "unmanaged" -> paths(m.unmanaged),
          "managed" -> paths(m.managed),
          "sources" -> paths(m.sources),
          "scalacOptions" -> Json.Arr(m.options.map(Json.Str(_))),
          "flags" -> Json.Arr(options.flags(jvm).map(Json.Str(_))),
          "ignoredScalacOptions" -> Json.Arr(options.ignored.map(Json.Str(_))),
        ))
      val document = Json.Obj(Map(
        "project" -> Json.Str(self.project),
        "configuration" -> Json.Str(c.name),
        "root" -> Json.Str(root.toString),
        "modules" -> Json.Arr(modules.value.map(described)),
        "classpath" -> Json.Arr(classpath.map {
          case (path, Some((p, conf))) => Json.Obj(Map("path" -> Json.Str(path.toString), "project" -> Json.Str(p.project), "configuration" -> Json.Str(conf)))
          case (path, None) => Json.Obj(Map("path" -> Json.Str(path.toString)))
        }),
      ))
      IO.write(out, render(document) + "\n", UTF_8)
      streams.value.log.info(s"teq: wrote $out")
      out
    }
  }

  private def render(v: Json.Value): String = v match
    case Json.Obj(fields) => fields.toSeq.sortBy(_._1).map((k, x) => s"${Json.string(k)}:${render(x)}").mkString("{", ",", "}")
    case Json.Arr(items) => items.map(render).mkString("[", ",", "]")
    case Json.Str(s) => Json.string(s)
    case Json.Num(n) => n
    case Json.Bool(b) => b.toString
    case Json.Null => "null"
