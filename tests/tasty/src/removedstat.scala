// Statements the reference removes after pickling (PureStats, Compiler.scala:116, runs on trees the Pickler,
// Compiler.scala:50, has already written): TASTy keeps each one: an object's name, a literal, a parameter, a
// function literal as statements of a block; the pure result of an ascribed block; a value a function
// literal's Unit result discards.
package fix.removedstat

object RsQuiet
class RsUse:
  def named(): Unit = { RsQuiet; println("kept") }
  def literal(): Unit = { 1; println("kept") }
  def parameter(x: Int): Unit = { x; println("kept") }
  def function(): Unit = { ((y: Int) => y); println("kept") }
  def ascribed(): Unit = { ({ println("in"); RsQuiet }: Any); println("kept") }
  def discarded: () => Unit = () => RsQuiet
