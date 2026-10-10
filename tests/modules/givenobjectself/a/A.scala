// A parameterless structural given is a module (`Parsers.givenDef`): its name reads its class, every member of
// the body visible downstream (`top.extra`), and inside its body the name and a search for its type are its `this`
// (`tpd.ref`), so a val that reads it while it is made gets the instance.
package gosa

trait T:
  val self: T
  def n: Int

given top: T with
  val self: T = summon[T]
  def n = 1
  def extra: String = "top"

object Holder:
  given held: T with
    val self: T = held
    def n = 2
    def extra: String = "held"
