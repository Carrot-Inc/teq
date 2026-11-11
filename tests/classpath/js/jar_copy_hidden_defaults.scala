// jars: scala-library defaults-lib
// targets: js interp
// A jar's inline bodies that copy a case class with parameters that are not public
// (tests/support/defaults_lib.scala's `SettingsOps`): the copy keeps the private, the protected and the
// package-private parameter, read by the class's own getters.
import defaultslib.*

@main def main(): Unit =
  val made = Settings.make
  println(SettingsOps.flagged(made).show)
  println(SettingsOps.scoped(made, 7).show)
