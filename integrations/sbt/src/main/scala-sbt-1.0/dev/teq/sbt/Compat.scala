package dev.teq.sbt

import java.io.File
import java.nio.file.Path

import org.scalajs.linker.interface.Report
import org.scalajs.sbtplugin.ScalaJSPlugin.autoImport.scalaJSLinkerOutputDirectory
import sbt.*
import sbt.Keys.*
import sbt.librarymanagement.ivy.{Credentials => IvyCredentials, DirectCredentials, FileCredentials}
import sbt.util.Logger
import xsbti.FileConverter

/** What the plugin reads of sbt's API where sbt 1 and sbt 2 differ, sbt 1's side (sbt 2's is
  * src/main/scala-sbt-2's): the members sbt 2's side has, with the same meaning. sbt 1 names a classpath
  * entry and an artifact's path as a file, and a resolved module, its artifact and its configuration as
  * typed attributes of the entry; it has no `platform` key, no `taskDefinitionKey` on a task but in its
  * info, and caches no task. */
private[sbt] object Compat {
  /** A path sbt names, and a classpath entry: a file. */
  type Ref = File
  type Entry = File

  def path(ref: Ref, converter: FileConverter): Path = ref.toPath
  def file(ref: Ref, converter: FileConverter): File = ref

  /** A resolved module's organization, name and revision, from a classpath entry's attributes. */
  def module(entry: Attributed[?]): Option[(String, String, String)] =
    entry.get(Keys.moduleID.key).map(m => (m.organization, m.name, m.revision))
  /** The entry's artifact's name, URL and classifier ("" for none, as sbt 2's JSON gives them). */
  def artifactName(entry: Attributed[?]): Option[String] = entry.get(Keys.artifact.key).map(_.name)
  def artifactUrl(entry: Attributed[?]): Option[String] = entry.get(Keys.artifact.key).map(_.url.fold("")(_.toString))
  def classifier(entry: Attributed[?]): Option[String] = entry.get(Keys.artifact.key).map(_.classifier.getOrElse(""))
  /** The configuration a classpath takes the entry through. */
  def configuration(entry: Attributed[?]): Option[String] = entry.get(Keys.configuration.key).map(_.name)

  /** The build's settings, as `settingsData` gives them. */
  type Settings = sbt.internal.util.Settings[Scope]

  /** A setting's or a task's value in the settings, by its scoped key. */
  def lookup(data: Settings, key: ScopedKey[?]): Option[Any] = data.get(key.scope, key.key)

  /** The key a task was defined under. */
  def definition(task: Task[?]): Option[ScopedKey[?]] = task.info.get(Keys.taskDefinitionKey)

  /** The project's platform as sbt 2's `platform` key names it, from the plugins that make it (sbt 1 has no
    * such key): `sjs1` for Scala.js, `native` for Scala Native, else `jvm`. */
  val platform: Def.Initialize[String] = Def.setting {
    val labels = thisProject.value.autoPlugins.map(_.label).toSet
    if (labels("org.scalajs.sbtplugin.ScalaJSPlugin")) "sjs1"
    else if (labels("scala.scalanative.sbtplugin.ScalaNativePlugin")) "native"
    else "jvm"
  }

  /** The credentials sbt holds (`allCredentials`), as host, user and password, a file's read as sbt
    * reads it; one that cannot be read passed over with a warning, as sbt passes it over. */
  type Credentials = IvyCredentials
  val Credentials = IvyCredentials
  def hosts(credentials: Seq[Credentials], log: Logger): Seq[(String, String, String)] =
    credentials.flatMap {
      case c: DirectCredentials => Some((c.host, c.userName, c.passwd))
      case c: FileCredentials =>
        IvyCredentials.loadCredentials(c.path) match {
          case Right(loaded) => Some((loaded.host, loaded.userName, loaded.passwd))
          case Left(why) =>
            log.warn(s"teq: $why, ignoring it")
            None
        }
    }

  /** The task's value given to `f`, whose task follows. */
  def flatMapTask[A, B](task: Task[A])(f: A => Def.Initialize[Task[B]]): Def.Initialize[Task[B]] =
    Def.taskDyn(f(Def.value(task).value))

  /** A linker's report with the directory it wrote, under the attribute `fastLinkJSOutput` and
    * `fullLinkJSOutput` read it from: sbt 1 keeps it as the file, under the setting's own key. */
  def linkReport(report: Report, out: File): Attributed[Report] =
    Attributed.blank(report).put(scalaJSLinkerOutputDirectory.key, out)

  /** A task whose work is outside sbt's knowledge: sbt 1 caches no task. */
  def uncached[A](key: TaskKey[A], task: Def.Initialize[Task[A]]): Setting[Task[A]] =
    key := task.value

  /** zinc's second inputs, sbt 2's alone: zinc's `setup.extra` carries teq's identity on both. */
  def compileInputsSettings: Seq[Setting[?]] = Nil
}
