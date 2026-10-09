package coa

// Top-level case objects and one nested in an object, as scalac writes them: the module class `Stop$` is the
// object's `Mirror.Singleton`, `Stop` the mirror class of its static forwarders, and no companion of a case class
// (item 9 of the JVM ABI alignment).
sealed trait Signal
case object Stop extends Signal
case object Go extends Signal
object Lights:
  case object Amber extends Signal
