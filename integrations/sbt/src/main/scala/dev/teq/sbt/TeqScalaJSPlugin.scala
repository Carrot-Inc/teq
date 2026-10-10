package dev.teq.sbt

import java.io.File

import scala.collection.mutable

import org.scalajs.jsenv.Input
import org.scalajs.linker.interface.{ModuleInitializer, ModuleKind}
import org.scalajs.linker.interface.unstable.ReportImpl
import org.scalajs.testing.adapter.TestAdapterInitializer
import org.scalajs.sbtplugin.ScalaJSPlugin
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport.*
import sbt.*
import sbt.Keys.*

import TeqPlugin.autoImport.*

/** A Scala.js project under `teqCompiler`: `fastLinkJS` and `fullLinkJS` are answered from teq's
  * output, written where the linker would write its own (the task's
  * `scalaJSLinkerOutputDirectory`), so that a build that reads the setting, one that reads the
  * report's attribute (`fastLinkJSOutput`, the stock Scala.js vite plugin) and `sbt ~fastLinkJS`
  * run unmodified over it. The link tasks return the report the linker would: one public ES
  * module named `main` in `main.js` (the dev build's stub beside teq's `main.mjs`, the full
  * build's one file), attributed with the directory. Without the toggle the linker links, as
  * the directory's writer for as long as it does, after what teq left there is gone with the
  * linker's own record of its last link, which would otherwise answer for files that are no
  * longer there. The `Test` configuration's two link tasks are answered the same way, by the
  * link of the test sources whose entry point starts Scala.js's test bridge, so that
  * sbt-scalajs's `test`, `testOnly` and `testQuick` run the suites teq compiled under its own
  * test adapter; `compile` of both configurations is teq's check (`TeqPlugin.compilerSettings`). */
object TeqScalaJSPlugin extends AutoPlugin {
  override def trigger = allRequirements
  override def requires = TeqPlugin && ScalaJSPlugin

  override def projectSettings: Seq[Setting[?]] =
    Seq(fastLinkJS -> false, fullLinkJS -> true).flatMap { case (task, full) =>
      val directory = Def.setting((Compile / task / scalaJSLinkerOutputDirectory).value)
      val teq = (if (full) TeqPlugin.fullLink(directory) else TeqPlugin.link(directory)).map(report)
      Seq(
        Compat.uncached(Compile / task, Def.taskIf {
          if (teqCompiler.value) teq.value
          else stock(Compile, task, directory).value
        }),
      ) ++ inConfig(Compile)(TeqPlugin.watched(task, directory, Def.setting(teqCompiler.value && !full)))
    } ++ Seq(fastLinkJS -> false, fullLinkJS -> true).flatMap { case (task, full) =>
      val directory = Def.setting((Test / task / scalaJSLinkerOutputDirectory).value)
      val teq = (if (full) TeqPlugin.testFullLink(directory) else TeqPlugin.testLink(directory)).map(report)
      Seq(
        Compat.uncached(Test / task, Def.taskIf {
          if (teqCompiler.value) teq.value
          else stock(Test, task, directory).value
        }),
      )
    } ++ scalaJSTest

  /** The test run's facts the export records (`Export.teqExportScalaJSTest`): the test link's module kind and
    * the JavaScript environment, which teq reads (docs/TOOLING.md, "The lockfile and the launchers"), and the
    * reasons teq cannot run the suites as sbt-scalajs does (`Export.teqExportScalaJSTestRefusals`, `testJs`). */
  private val scalaJSTest: Seq[Setting[?]] = Seq(
    Export.teqExportScalaJSTest := testJs.value.fields,
    Export.teqExportScalaJSTestRefusals := testJs.value.refusals,
  )

  /** A JavaScript environment as the lock records it: its kind (`node` for Node.js, `jsdom` for jsdom's,
    * else the environment's class, lowercased) and, from its configuration where the environment keeps one
    * (Node.js's and jsdom's, read as their `Config`s give them), the executable, its arguments and its
    * environment variables. */
  private[sbt] def jsEnvJson(env: AnyRef): Json.Value = {
    val kind = env.getClass.getName match {
      case "org.scalajs.jsenv.nodejs.NodeJSEnv" => "node"
      case "org.scalajs.jsenv.jsdomnodejs.JSDOMNodeJSEnv" => "jsdom"
      case other => other.split('.').last.toLowerCase
    }
    val config: Option[AnyRef] =
      try {
        val field = env.getClass.getDeclaredField("config")
        field.setAccessible(true)
        Option(field.get(env))
      }
      catch { case scala.util.control.NonFatal(_) => None }
    def read[T](name: String): Option[T] = config.flatMap(c =>
      try Option(c.getClass.getMethod(name).invoke(c).asInstanceOf[T]) catch { case scala.util.control.NonFatal(_) => None })
    Json.Obj(Map(
      "kind" -> Json.Str(kind),
      "args" -> Json.Arr(read[Seq[String]]("args").getOrElse(Nil).map(Json.Str(_))),
      "env" -> Json.Obj(read[Map[String, String]]("env").getOrElse(Map.empty).map { case (k, v) => k -> Json.Str(v) }),
    ) ++ read[String]("executable").map("executable" -> Json.Str(_)))
  }

  /** How sbt-scalajs runs the `Test` configuration's link, for teqExportAll (`TestJs.of`): the
    * module kind of `Test / scalaJSLinkerConfig`, whose module split and ES features shape the
    * linker's files and syntax alone, which teq's one file of ES2015 has its own of, and `Test /
    * jsEnv`. A module initializer of the build's own before the tests (sbt-scalajs adds the
    * bridge's) and an input of its own before the link (sbt-scalajs adds the link's, which teq's
    * run supplies; `TestJs.inputRefusals`) are refusals. The inputs added at the project's level
    * are read; a definition of `Test / jsEnvInput` other than sbt-scalajs's two (from its
    * `ScalaJSPluginInternal.scala`) is told by where it stands, its inputs only linking would give. */
  private lazy val testJs: Def.Initialize[Task[TestJs]] = Def.task {
    val linker = (Test / scalaJSLinkerConfig).value
    val env = (Test / jsEnv).value
    val initializers = (Test / scalaJSModuleInitializers).value
    val inputs = jsEnvInput.value
    val root = (LocalRootProject / baseDirectory).value
    val testScope = Scope(Select(thisProjectRef.value), Select(ConfigKey(Test.name)), Zero, Zero)
    def relative(path: String): String = IO.relativize(root, new File(path)).getOrElse(path)
    val testDefinitions = Project.extract(state.value).structure.settings.collect {
      case s if s.key.key.label == jsEnvInput.key.label && s.key.scope == testScope => s.pos
    }.collect {
      case p: FilePosition if !TestJs.fromScalaJS(p.path) => s"${relative(p.path)}:${p.startLine}"
      case NoPosition => "a place sbt does not tell"
    }
    val described = inputs.map {
      case Input.Script(path) => s"the script ${relative(path.toString)}"
      case Input.ESModule(path) => s"the ES module ${relative(path.toString)}"
      case Input.CommonJSModule(path) => s"the CommonJS module ${relative(path.toString)}"
      case other => other.toString
    }
    val refusals = mutable.ArrayBuffer.empty[String]
    val bridge = ModuleInitializer.fingerprint(ModuleInitializer.mainMethod(TestAdapterInitializer.ModuleClassName, TestAdapterInitializer.MainMethodName))
    val own = initializers.map(ModuleInitializer.fingerprint).filterNot(_ == bridge)
    if (own.nonEmpty) refusals += s"Test / scalaJSModuleInitializers runs ${own.mkString(", ")} before the tests, which teq's test link does not"
    refusals ++= TestJs.inputRefusals(described, testDefinitions)
    val kind = linker.moduleKind match {
      case ModuleKind.NoModule => "NoModule"
      case ModuleKind.CommonJSModule => "CommonJSModule"
      case ModuleKind.ESModule => "ESModule"
    }
    TestJs.of(kind, env, root, refusals.toSeq)
  }

  /** The Scala.js linker's own link of the configuration, as the directory's writer. */
  private def stock(config: Configuration, task: TaskKey[Attributed[org.scalajs.linker.interface.Report]], directory: Def.Initialize[File]) =
    Def.setting((directory.value, linking(config, task).value)).zipWith((config / task).dependsOn(linker(config, task, directory))) {
      case ((dir, by), link) => link.andFinally(Directory.done(dir, by))
    }

  private def linking(config: Configuration, task: TaskKey[?]): Def.Initialize[String] =
    Def.setting {
      val scope = if (config == Compile) "" else s"${config.name}/"
      s"the Scala.js linker (${thisProject.value.id}/$scope${task.key.label})"
    }

  /** Before the Scala.js linker links: the directory is this process's and the linker's to
    * write until the link has ended (`Directory.done`), and what teq wrote there is gone. */
  private def linker(config: Configuration, task: TaskKey[?], directory: Def.Initialize[File]): Def.Initialize[Task[Unit]] = Def.task {
    val log = streams.value.log
    val record = (config / task / streams).value.cacheDirectory / "linking-report.bin"
    Directory.claim(directory.value, linking(config, task).value, teqLinkWait.value, log) { dir =>
      if (Directory.takeOver(dir, None, log)) IO.delete(record)
    }
  }

  /** The linker's report of a directory teq served: one ES module, `main.js`, and the directory
    * under the attribute `fastLinkJSOutput` and `fullLinkJSOutput` read it from. */
  private def report(out: File): Attributed[org.scalajs.linker.interface.Report] = {
    val module = new ReportImpl.ModuleImpl("main", "main.js", None, ModuleKind.ESModule)
    Compat.linkReport(new ReportImpl(List(module)), out)
  }
}
