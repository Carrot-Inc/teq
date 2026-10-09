//> using file ../Helper.scala
//> using file Cycle.scala
// Included by tests/using/chain.scala, its own directives followed by teq interp (Scala CLI ignores
// them), a cycle back to this file read once.
object Chained:
  val value = "chained to " + Helper.value + " and " + Cycle.value
