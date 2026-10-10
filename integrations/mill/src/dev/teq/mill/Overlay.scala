package dev.teq.mill

import dev.teq.sbt.{Json, TeqCommand}

/** `teq.mill.json` at the build's root (docs/TOOLING.md, "The export of a Mill build"): what a Mill build
  * declares for teq that Mill cannot tell the export, sbt-teq's settings' twins. One of the build's inputs.
  *
  * {{{
  * {
  *   "buildTool": true,                 // teqBuildTool: the lock at the root, with the launchers
  *   "snapshots": false,                // teqExportSnapshots: false under buildTool, else true
  *   "teq": "0.1.8",                    // teqVersion: the compiler the lock pins (the plugin's by default)
  *   "releases": "https://...",         // teqReleases: where the compiler's releases are
  *   "inputs": [".tyrian-version"],     // files the build reads besides its scripts
  *   "modules": {
  *     "app": {
  *       "generators": [{"run": ["teq", "interp", "gen.scala", "--"], "inputs": ["labels.txt"]}],
  *       "mainClasses": ["app.Tool"],   // teqMainClasses
  *       "runAliases": {"tool": "app.Tool"},
  *       "devCommand": ["npm", "run", "dev"],
  *       "test": "app.integration"      // the module that is app's test configuration, not app.test
  *     }
  *   }
  * }
  * }}}
  *
  * A module's `generators` stand for its `generatedSources`, which the export never runs: commands that write
  * its sources (sbt-teq's `TeqCommand`), which teq runs as Mill would run the task. */
private[mill] final case class Overlay(
  buildTool: Boolean = false,
  snapshots: Option[Boolean] = None,
  teq: Option[String] = None,
  releases: Option[String] = None,
  inputs: Seq[String] = Nil,
  modules: Map[String, Overlay.Module] = Map.empty,
):
  def module(id: String): Overlay.Module = modules.getOrElse(id, Overlay.Module())
  /** Whether the export may name SNAPSHOT and dynamic versions: as declared, else without the driver alone. */
  def allowsSnapshots: Boolean = snapshots.getOrElse(!buildTool)

private[mill] object Overlay:
  val FileName = "teq.mill.json"

  final case class Module(
    generators: Seq[TeqCommand] = Nil,
    mainClasses: Seq[String] = Nil,
    runAliases: Map[String, String] = Map.empty,
    devCommand: Seq[String] = Nil,
    test: Option[String] = None,
  )

  private val Keys = Set("buildTool", "snapshots", "teq", "releases", "inputs", "modules")
  private val ModuleKeys = Set("generators", "mainClasses", "runAliases", "devCommand", "test")
  private val CommandKeys = Set("run", "inputs", "cwd", "outputs")

  /** The overlay at the build's root, the default one without a file; a file that is no overlay, why. */
  def read(root: os.Path): Either[String, Overlay] =
    val file = root / FileName
    if !os.isFile(file) then Right(Overlay())
    else
      try parse(os.read(file)).left.map(why => s"$FileName: $why")
      catch case e: Json.Malformed => Left(s"$FileName is not JSON: ${e.getMessage}")

  def parse(text: String): Either[String, Overlay] =
    val problems = Seq.newBuilder[String]
    def strings(v: Json.Value, where: String): Seq[String] = v match
      case Json.Arr(items) if items.forall(_.isInstanceOf[Json.Str]) => items.map(_.str)
      case Json.Null => Nil
      case _ =>
        problems += s"$where is not a list of strings"
        Nil
    def string(v: Json.Value, where: String): Option[String] = v match
      case Json.Str(s) => Some(s)
      case Json.Null => None
      case _ =>
        problems += s"$where is not a string"
        None
    def boolean(v: Json.Value, where: String): Option[Boolean] = v match
      case Json.Bool(b) => Some(b)
      case Json.Null => None
      case _ =>
        problems += s"$where is not true or false"
        None
    def fields(v: Json.Value, where: String, known: Set[String]): Map[String, Json.Value] = v match
      case Json.Obj(fs) =>
        for k <- fs.keys.toSeq.sorted if !known(k) do problems += s"$where has no field $k (it takes ${known.toSeq.sorted.mkString(", ")})"
        fs
      case Json.Null => Map.empty
      case _ =>
        problems += s"$where is not an object"
        Map.empty
    val root = fields(Json.parse(text), "the overlay", Keys)
    val get = (k: String) => root.getOrElse(k, Json.Null)
    val modules = (get("modules") match
      case Json.Obj(fs) => fs
      case Json.Null => Map.empty
      case _ =>
        problems += "modules is not an object"
        Map.empty
    ).map { (id, v) =>
      val where = s"modules.$id"
      val m = fields(v, where, ModuleKeys)
      val f = (k: String) => m.getOrElse(k, Json.Null)
      val generators = f("generators") match
        case Json.Arr(items) => items.zipWithIndex.flatMap { (g, i) =>
          val at = s"$where.generators[$i]"
          val c = fields(g, at, CommandKeys)
          val run = strings(c.getOrElse("run", Json.Null), s"$at.run")
          if run.isEmpty then
            problems += s"$at has no run"
            None
          else Some(TeqCommand(run, strings(c.getOrElse("inputs", Json.Null), s"$at.inputs"), string(c.getOrElse("cwd", Json.Null), s"$at.cwd").getOrElse("."), strings(c.getOrElse("outputs", Json.Null), s"$at.outputs")))
        }
        case Json.Null => Nil
        case _ =>
          problems += s"$where.generators is not a list"
          Nil
      val aliases = fields(f("runAliases"), s"$where.runAliases", Set.empty[String] ++ (f("runAliases") match { case Json.Obj(fs) => fs.keySet; case _ => Set.empty }))
        .flatMap((k, v) => string(v, s"$where.runAliases.$k").map(k -> _))
      id -> Module(generators, strings(f("mainClasses"), s"$where.mainClasses"), aliases, strings(f("devCommand"), s"$where.devCommand"), string(f("test"), s"$where.test"))
    }
    val inputs = strings(get("inputs"), "inputs")
    for input <- inputs if input.isEmpty || input.startsWith("/") || input.startsWith("\\") || (input.length > 1 && input(1) == ':') do
      problems += s"inputs names $input, which is no path relative to the build's root"
    val overlay = Overlay(
      boolean(get("buildTool"), "buildTool").getOrElse(false),
      boolean(get("snapshots"), "snapshots"),
      string(get("teq"), "teq"),
      string(get("releases"), "releases"),
      inputs,
      modules,
    )
    problems.result() match
      case Seq() => Right(overlay)
      case found => Left(found.mkString("; "))
