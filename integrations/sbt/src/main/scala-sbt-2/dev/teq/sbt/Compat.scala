package dev.teq.sbt

import java.io.File
import java.nio.file.Path

import org.scalajs.linker.interface.Report
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport.scalaJSLinkerOutputDirectory
import sbt.*
import sbt.Keys.*
import sbt.internal.librarymanagement.ivy.IvyCredentials
import sbt.internal.util.StringAttributeKey
import sbt.util.Logger
import xsbti.FileConverter

import TeqPlugin.autoImport.*

/** What the plugin reads of sbt's API where sbt 2 and sbt 1 differ, sbt 2's side (sbt 1's is
  * src/main/scala-sbt-1.0's): the shared sources call these members alone, with the same meaning on
  * both. sbt 2 names a classpath entry and an artifact's path as a virtual file, and a resolved
  * module, its artifact and its configuration as JSON strings among the entry's attributes. */
private[sbt] object Compat {
  /** A path sbt names, and a classpath entry: a virtual file. */
  type Ref = xsbti.VirtualFileRef
  type Entry = xsbti.HashedVirtualFileRef

  def path(ref: Ref, converter: FileConverter): Path = converter.toPath(ref)
  def file(ref: Ref, converter: FileConverter): File = path(ref, converter).toFile

  /** A resolved module's organization, name and revision, from a classpath entry's attributes. */
  def module(entry: Attributed[?]): Option[(String, String, String)] =
    entry.get(Keys.moduleIDStr).map(Json.parse).map(m => (m("organization").str, m("name").str, m("revision").str))
  /** The entry's artifact's name, URL and classifier ("" for none). */
  def artifactName(entry: Attributed[?]): Option[String] = artifact(entry, "name")
  def artifactUrl(entry: Attributed[?]): Option[String] = artifact(entry, "url")
  def classifier(entry: Attributed[?]): Option[String] = artifact(entry, "classifier")
  private def artifact(entry: Attributed[?], field: String): Option[String] = entry.get(Keys.artifactStr).map(Json.parse(_)(field).str)
  /** The configuration a classpath takes the entry through. */
  def configuration(entry: Attributed[?]): Option[String] = entry.get(Keys.configurationStr)

  /** The build's settings, as `settingsData` gives them. */
  type Settings = Def.Settings

  /** A setting's or a task's value in the settings, by its scoped key. */
  def lookup(data: Settings, key: ScopedKey[?]): Option[Any] = data.get(key)

  /** The key a task was defined under. */
  def definition(task: Task[?]): Option[ScopedKey[?]] = task.get(Keys.taskDefinitionKey)

  /** The project's platform, sbt's own key: `jvm`, `sjs1` for Scala.js. */
  val platform: Def.Initialize[String] = Keys.platform

  /** The credentials sbt holds (`allCredentials`), as host, user and password, a file's read as sbt
    * reads it; one that cannot be read passed over with a warning, as sbt passes it over. */
  type Credentials = sbt.librarymanagement.Credentials
  val Credentials = sbt.librarymanagement.Credentials
  def hosts(credentials: Seq[Credentials], log: Logger): Seq[(String, String, String)] =
    credentials.flatMap {
      case c: Credentials.DirectCredentials => Some((c.host, c.userName, c.passwd))
      case c: Credentials.FileCredentials =>
        IvyCredentials.loadCredentials(c.path) match {
          case Right(loaded) => Some((loaded.host, loaded.userName, loaded.passwd))
          case Left(why) =>
            log.warn(s"teq: $why, ignoring it")
            None
        }
    }

  /** The task's value given to `f`, whose task follows. */
  def flatMapTask[A, B](task: Task[A])(f: A => Def.Initialize[Task[B]]): Def.Initialize[Task[B]] =
    Def.value[Task[A]](task).flatMapTask(f)

  /** A linker's report with the directory it wrote, under the attribute `fastLinkJSOutput` and
    * `fullLinkJSOutput` read it from: sbt 2 keeps it as the path's string. */
  def linkReport(report: Report, out: File): Attributed[Report] =
    Attributed.blank(report).put(StringAttributeKey(scalaJSLinkerOutputDirectory.key.label), out.getAbsolutePath)

  /** A task sbt 2 would cache by its inputs, whose work is outside sbt's knowledge (a generator's program, zinc's
    * compiler, a link), declared uncached. */
  def uncached[A](key: TaskKey[A], task: Def.Initialize[Task[A]]): Setting[Task[A]] =
    key := Def.uncached(task.value)

  /** zinc's second inputs, which carry sbt 2's compile cache's key: teq's identity added under `teqCompiler`, so
    * that a change of it compiles everything again (sbt 1 has none, zinc's `setup.extra` carrying the identity). */
  def compileInputsSettings: Seq[Setting[?]] = Seq(
    compile / compileInputs2 := Def.uncached(Def.taskIf {
      if (teqCompiler.value) {
        val inputs = (compile / compileInputs2).value
        inputs.copy(incrementalOptions = inputs.incrementalOptions :+ ("teq" -> teqCompilerCommand.value.identity))
      }
      else (compile / compileInputs2).value
    }.value),
  )
}
