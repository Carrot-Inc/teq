package dev.teq.sbt

import sbtbuildinfo.{Entry, PluginCompat}

/** sbt-buildinfo's keys as `BuildInfoGenerator` reads them, sbt 2's side (sbt 1's is src/main/scala-sbt-1.0's):
  * sbt-buildinfo's `Entry` for sbt 2 is an enum of public cases. Reached only for a project that enables it. */
private[sbt] object BuildInfoCompat {
  type Key = Entry[?]

  private def root(entry: Key): Key = entry match {
    case mapped: Entry.Mapped[?, ?] => root(mapped.from)
    case other => other
  }

  /** The keys sbt-buildinfo evaluates for the export: settings and constants, maybe mapped. */
  def evaluable(entry: Key): Boolean = root(entry) match {
    case _: Entry.Setting[?] | _: Entry.Constant[?] => true
    case _ => false
  }

  /** The key itself an action: its name and its value's manifest. */
  def action(entry: Key): Option[(String, PluginCompat.Manifest[?])] = entry match {
    case action: Entry.Action[?] => Some((action.name, action.manifest))
    case _ => None
  }

  /** The name of the action the key is, maybe mapped. */
  def rootAction(entry: Key): Option[String] = root(entry) match {
    case action: Entry.Action[?] => Some(action.name)
    case _ => None
  }

  /** The name of the key, an action's or a task's, an unevaluated key's reason names. */
  def keyName(entry: Key): String = root(entry) match {
    case action: Entry.Action[?] => action.name
    case task: Entry.Task[?] => task.scoped.key.label
    case _ => entry.toString
  }

  /** The type constructor of a manifest and its arguments, as sbt-buildinfo's Scala renderer reads them. */
  def typeExpression(manifest: PluginCompat.Manifest[?]): (String, List[PluginCompat.Manifest[?]]) =
    PluginCompat.TypeExpression.unapply(manifest)
}
