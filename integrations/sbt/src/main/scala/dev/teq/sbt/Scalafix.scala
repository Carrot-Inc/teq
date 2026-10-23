package dev.teq.sbt

import java.io.File

import sbt.*
import sbt.Keys.*

import scalafix.sbt.ScalafixPlugin.autoImport.{scalafix, scalafixConfig}

/** What sbt-scalafix does in the configurations it covers, compile's and test's, for the export's `scalafix`
  * records that `teq fix` follows: the configuration file it resolves there (`scalafixConfig`, none for the root's
  * `.scalafix.conf`) and whether it skips the configuration (`scalafix / skip`). sbt-scalafix is a dependency of the
  * plugin's compile alone: `of` is reached only for a project that enables `ScalafixPlugin`. */
private[sbt] object Scalafix {
  val Label = "scalafix.sbt.ScalafixPlugin"

  /** Each covered configuration's file and skip, by its name. */
  type Settings = Map[String, (Option[File], Boolean)]

  /** A project without the plugin in a build that has it: sbt-scalafix runs in none of its configurations. */
  val Disabled: Settings = Map("compile" -> (None, true), "test" -> (None, true))

  def of: Def.Initialize[Task[Settings]] = Def.task {
    Map(
      "compile" -> ((Compile / scalafixConfig).value, (Compile / scalafix / skip).value),
      "test" -> ((Test / scalafixConfig).value, (Test / scalafix / skip).value),
    )
  }
}
