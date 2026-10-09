package dev.teq.sbt

import java.io.File
import java.nio.file.Path

import sbt.*
import sbt.Keys.*
import sbtbuildinfo.{BuildInfo, BuildInfoResult, Entry, PluginCompat}
import sbtbuildinfo.BuildInfoPlugin.autoImport.*

/** sbt-buildinfo's object as the export's `buildInfo` generator. sbt-buildinfo is a dependency of
  * the plugin's compile alone: this object is reached only for a project that enables it. */
private[sbt] object BuildInfoGenerator:
  /** The generator of the project's Compile `BuildInfo`: the keys sbt-buildinfo computes in the
    * build as `static` values, and as `dynamic` ones what `teq` computes: `gitSha`, an action
    * of that name, and a class directory of the build, by its project and configuration; the
    * members in sbt-buildinfo's order. Only the kinds `teq` writes as sbt-buildinfo does are
    * taken, by the type sbt-buildinfo gives each: a string, an `Int`, a `Boolean`, strings. The
    * actions and tasks are not run: their code is the build's, which the export leaves alone. */
  val of: Def.Initialize[Task[Either[Seq[String], Json.Value]]] = Def.taskDyn {
    evaluated((Compile / buildInfoKeys).value.distinct, thisProjectRef.value, state.value)
  }

  private def root(entry: Entry[?]): Entry[?] = entry match
    case mapped: Entry.Mapped[?, ?] => root(mapped.from)
    case other => other

  /** The keys sbt-buildinfo evaluates for the export: settings and constants, maybe mapped. */
  private def evaluable(entry: Entry[?]): Boolean = root(entry) match
    case _: Entry.Setting[?] | _: Entry.Constant[?] => true
    case _ => false

  private def evaluated(keys: Seq[Entry[?]], project: ProjectRef, state: State): Def.Initialize[Task[Either[Seq[String], Json.Value]]] =
    val results = keys.filter(evaluable).map(key => BuildInfo.results(Seq(key), Nil, project, state)).join
    Def.value[Task[Seq[Seq[BuildInfoResult]]]](results).flatMapTask(results => generator(keys, results))

  private def generator(keys: Seq[Entry[?]], evaluated: Seq[Seq[BuildInfoResult]]): Def.Initialize[Task[Either[Seq[String], Json.Value]]] = Def.task {
    val name = thisProjectRef.value.project
    val data = settingsData.value
    val buildRoot = Export.rootOf((LocalRootProject / baseDirectory).value)
    val options = (Compile / buildInfoOptions).value
    val renderer = (Compile / buildInfoRenderer).value
    val packageName = (Compile / buildInfoPackage).value
    val objectName = (Compile / buildInfoObject).value
    val packagePath = (Compile / buildInfoUsePackageAsPath).value
    val classDirectories: Map[Path, (String, String)] = loadedBuild.value.allProjectRefs.map(_._1).flatMap { p =>
      Seq(Compile, Test).flatMap(c => (p / c / classDirectory).get(data).map(dir => dir.toPath.toAbsolutePath.normalize -> (p.project, c.name)))
    }.toMap
    def member(r: BuildInfoResult): Either[String, (String, Either[Json.Value, Json.Value])] =
      (typeOf(r.manifest), r.value) match
        case (Some("java.io.File"), file: File) =>
          classDirectories.get(file.toPath.toAbsolutePath.normalize) match
            case Some((project, configuration)) =>
              Right(r.identifier -> Left(Json.Obj(Map("classDirectory" -> Json.Obj(Map("project" -> Json.Str(project), "configuration" -> Json.Str(configuration)))))))
            // Relative to the build's root, and outside it unnamed, so that the reason the lock
            // records is the same bytes on every machine and in every checkout.
            case None =>
              val path = file.toPath.toAbsolutePath.normalize
              val what = if path.startsWith(buildRoot) then s"the file ${Export.relativeTo(buildRoot, file)}" else "a file outside the build's root"
              Left(s"the BuildInfo key ${r.identifier} is $what, which is no class directory of the build")
        case (Some("String"), s: String) => Right(r.identifier -> Right(Json.Str(s)))
        case (Some("scala.Int"), n: Int) => Right(r.identifier -> Right(Json.Num(n.toString)))
        case (Some("scala.Boolean"), b: Boolean) => Right(r.identifier -> Right(Json.Bool(b)))
        case (Some("scala.collection.immutable.Seq[String]"), items: Seq[?]) if items.forall(_.isInstanceOf[String]) =>
          Right(r.identifier -> Right(Json.Arr(items.map(i => Json.Str(i.toString)))))
        case (tpe, value) =>
          Left(s"the BuildInfo key ${r.identifier} is a ${tpe.getOrElse(value.getClass.getName)}, which teq does not write as sbt-buildinfo does: a string, an Int, a Boolean or strings")
    val results = keys.filter(evaluable).zip(evaluated).toMap
    val members = keys.flatMap { key =>
      key match
        case action: Entry.Action[?] if action.name == "gitSha" && typeOf(action.manifest).contains("String") => Seq(Right(action.name -> Left(Json.Str("gitSha"))))
        case _ if evaluable(key) => results(key).map(member)
        case other =>
          val what = root(other) match
            case action: Entry.Action[?] => s"the action ${action.name}"
            case _ => "a task"
          Seq(Left(s"the BuildInfo key ${keyName(other)} runs $what, which teq cannot: only an action gitSha of a string is computed outside sbt"))
    }
    val identifiers = members.collect { case Right((id, _)) => id }
    val refusals =
      Option.when(options.nonEmpty)(s"sbt-buildinfo's options ${options.mkString(", ")} are not teq's: it writes the plain object").toSeq ++
        Option.when(renderer.getClass.getSimpleName != "Scala3CaseObjectRenderer")(s"sbt-buildinfo renders BuildInfo with ${renderer.getClass.getSimpleName}, where teq writes Scala 3's case object") ++
        Option.when(identifiers.distinct.size != identifiers.size)(s"BuildInfo names ${identifiers.diff(identifiers.distinct).distinct.mkString(", ")} twice") ++
        members.collect { case Left(why) => why }
    if refusals.nonEmpty then Left(refusals)
    else
      val named = members.collect { case Right(member) => member }
      val directory = if packagePath then packageName.split('.').filter(_.nonEmpty).mkString("/") else "sbt-buildinfo"
      val output = Seq(Export.generatedDirectory(name), directory, s"$objectName.scala").filter(_.nonEmpty).mkString("/")
      Right(Json.Obj(Map(
        "kind" -> Json.Str("buildInfo"),
        "package" -> Json.Str(packageName),
        "object" -> Json.Str(objectName),
        "output" -> Json.Str(output),
        "members" -> Json.Arr(named.map((id, _) => Json.Str(id))),
        "static" -> Json.Obj(named.collect { case (id, Right(v)) => id -> v }.toMap),
        "dynamic" -> Json.Obj(named.collect { case (id, Left(v)) => id -> v }.toMap),
      )))
  }

  private def keyName(entry: Entry[?]): String = root(entry) match
    case action: Entry.Action[?] => action.name
    case task: Entry.Task[?] => task.scoped.key.label
    case _ => entry.toString

  /** The type sbt-buildinfo's Scala renderer declares for a manifest, of the kinds the export
    * carries; the type constructor of the others. */
  private def typeOf(manifest: PluginCompat.Manifest[?]): Option[String] =
    PluginCompat.TypeExpression.unapply(manifest) match
      case ("java.lang.String", Nil) => Some("String")
      case ("Int" | "scala.Int", Nil) => Some("scala.Int")
      case ("Boolean" | "scala.Boolean" | "java.lang.Boolean", Nil) => Some("scala.Boolean")
      case ("java.io.File" | "sbt.File", Nil) => Some("java.io.File")
      case ("scala.collection.Seq" | "scala.collection.immutable.Seq", List(item)) if typeOf(item).contains("String") => Some("scala.collection.immutable.Seq[String]")
      case (constructor, _) => Some(constructor)
