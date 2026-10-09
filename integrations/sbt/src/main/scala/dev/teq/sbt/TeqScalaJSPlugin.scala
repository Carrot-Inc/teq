package dev.teq.sbt

import java.io.File

import org.scalajs.linker.interface.ModuleKind
import org.scalajs.linker.interface.unstable.ReportImpl
import org.scalajs.sbtplugin.ScalaJSPlugin
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport.*
import sbt.*
import sbt.Keys.*
import sbt.internal.util.StringAttributeKey

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
object TeqScalaJSPlugin extends AutoPlugin:
  override def trigger = allRequirements
  override def requires = TeqPlugin && ScalaJSPlugin

  override def projectSettings: Seq[Setting[?]] =
    Seq(fastLinkJS -> false, fullLinkJS -> true).flatMap { (task, full) =>
      val directory = Def.setting((Compile / task / scalaJSLinkerOutputDirectory).value)
      val teq = (if full then TeqPlugin.fullLink(directory) else TeqPlugin.link(directory)).map(report)
      Seq(
        Compile / task := Def.uncached(Def.taskIf {
          if teqCompiler.value then teq.value
          else stock(Compile, task, directory).value
        }.value),
      ) ++ inConfig(Compile)(TeqPlugin.watched(task, directory, Def.setting(teqCompiler.value && !full)))
    } ++ Seq(fastLinkJS -> false, fullLinkJS -> true).flatMap { (task, full) =>
      val directory = Def.setting((Test / task / scalaJSLinkerOutputDirectory).value)
      val teq = (if full then TeqPlugin.testFullLink(directory) else TeqPlugin.testLink(directory)).map(report)
      Seq(
        Test / task := Def.uncached(Def.taskIf {
          if teqCompiler.value then teq.value
          else stock(Test, task, directory).value
        }.value),
      )
    }

  /** The Scala.js linker's own link of the configuration, as the directory's writer. */
  private def stock(config: Configuration, task: TaskKey[Attributed[org.scalajs.linker.interface.Report]], directory: Def.Initialize[File]) =
    Def.setting((directory.value, linking(config, task).value)).zipWith((config / task).dependsOn(linker(config, task, directory))) {
      case ((dir, by), link) => link.andFinally(Directory.done(dir, by))
    }

  private def linking(config: Configuration, task: TaskKey[?]): Def.Initialize[String] =
    Def.setting {
      val scope = if config == Compile then "" else s"${config.name}/"
      s"the Scala.js linker (${thisProject.value.id}/$scope${task.key.label})"
    }

  /** Before the Scala.js linker links: the directory is this process's and the linker's to
    * write until the link has ended (`Directory.done`), and what teq wrote there is gone. */
  private def linker(config: Configuration, task: TaskKey[?], directory: Def.Initialize[File]): Def.Initialize[Task[Unit]] = Def.task {
    val log = streams.value.log
    val record = (config / task / streams).value.cacheDirectory / "linking-report.bin"
    Directory.claim(directory.value, linking(config, task).value, teqLinkWait.value, log) { dir =>
      if Directory.takeOver(dir, None, log) then IO.delete(record)
    }
  }

  /** The linker's report of a directory teq served: one ES module, `main.js`, and the directory
    * under the attribute `fastLinkJSOutput` and `fullLinkJSOutput` read it from. */
  private def report(out: File): Attributed[org.scalajs.linker.interface.Report] =
    val module = new ReportImpl.ModuleImpl("main", "main.js", None, ModuleKind.ESModule)
    val report = new ReportImpl(List(module))
    Attributed.blank(report).put(StringAttributeKey(scalaJSLinkerOutputDirectory.key.label), out.getAbsolutePath)
