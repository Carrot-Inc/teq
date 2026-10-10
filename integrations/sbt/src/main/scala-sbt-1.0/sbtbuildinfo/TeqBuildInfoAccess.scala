package sbtbuildinfo

import BuildInfoKey.{Action, Constant, Entry, Mapped, Setting, Task}

/** sbt-buildinfo's keys for sbt 1, whose cases are its package's alone, as sbt-teq's export reads them
  * (dev.teq.sbt.BuildInfoCompat): in this package, which sbt-teq's sbt 1 jar adds to. */
object TeqBuildInfoAccess {
  private def root(entry: Entry[?]): Entry[?] = entry match {
    case mapped: Mapped[?, ?] => root(mapped.from)
    case other => other
  }

  def evaluable(entry: Entry[?]): Boolean = root(entry) match {
    case _: Setting[?] | _: Constant[?] => true
    case _ => false
  }

  def action(entry: Entry[?]): Option[(String, Manifest[?])] = entry match {
    case action: Action[?] => Some((action.name, action.manifest))
    case _ => None
  }

  def rootAction(entry: Entry[?]): Option[String] = root(entry) match {
    case action: Action[?] => Some(action.name)
    case _ => None
  }

  def keyName(entry: Entry[?]): String = root(entry) match {
    case action: Action[?] => action.name
    case task: Task[?] => task.scoped.key.label
    case _ => entry.toString
  }
}
