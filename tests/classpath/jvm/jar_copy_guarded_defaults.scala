// jars: scala-library defaults-lib
// A jar's inline bodies that copy a case class with a protected and a package-private parameter
// (tests/support/defaults_lib.scala's `SettingsOps` over `Guarded`), linked against the jar's class files: the
// copy keeps both, read as the class's own getters read them.
import defaultslib.*

@main def main(): Unit =
  val made = Guarded.make
  println(SettingsOps.flagged(made).show)
  println(SettingsOps.scoped(made, 7).show)
