package dev.teq.sbt

import sbtbuildinfo.{BuildInfoKey, PluginCompat, TeqBuildInfoAccess}

/** sbt-buildinfo's keys as `BuildInfoGenerator` reads them, sbt 1's side (sbt 2's is src/main/scala-sbt-2's):
  * sbt-buildinfo's `BuildInfoKey.Entry` for sbt 1 keeps its cases to its package, which `TeqBuildInfoAccess`
  * reads them in. Reached only for a project that enables it. */
private[sbt] object BuildInfoCompat {
  type Key = BuildInfoKey.Entry[?]

  /** The keys sbt-buildinfo evaluates for the export: settings and constants, maybe mapped. */
  def evaluable(entry: Key): Boolean = TeqBuildInfoAccess.evaluable(entry)

  /** The key itself an action: its name and its value's manifest. */
  def action(entry: Key): Option[(String, PluginCompat.Manifest[?])] = TeqBuildInfoAccess.action(entry)

  /** The name of the action the key is, maybe mapped. */
  def rootAction(entry: Key): Option[String] = TeqBuildInfoAccess.rootAction(entry)

  /** The name of the key, an action's or a task's, an unevaluated key's reason names. */
  def keyName(entry: Key): String = TeqBuildInfoAccess.keyName(entry)

  /** The type constructor of a manifest and its arguments, as sbt-buildinfo's Scala renderer reads them. */
  def typeExpression(manifest: PluginCompat.Manifest[?]): (String, List[PluginCompat.Manifest[?]]) =
    PluginCompat.TypeExpression.unapply(manifest).get
}
