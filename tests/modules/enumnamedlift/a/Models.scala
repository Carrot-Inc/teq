package enl

// An enum's companion holds its case values, so a reference to a case through the companion is
// idempotent, not pure (`TreeInfo.refPurity`: the module class is no `NoInits` class), and a
// named argument out of parameter order that reads one is lifted into a val before the call
// (`Applications`' reordering, `EtaExpansion.liftArgs`), whether the enum is this module's or an
// upstream product's.
enum Conn:
  case And, Or
// A `derives` clause gives the companion a definition of its own, the derived givens'; the value
// cases are its vals all the same.
trait Show[T]
object Show:
  def derived[T]: Show[T] = new Show[T] {}
enum Mode derives Show:
  case Fast, Slow
case class Group(items: List[Int], connective: Conn)
case class Run(items: List[Int], mode: Mode)
case class Key(name: String, keyType: Int, title: Option[String] = None, disabled: Boolean = false)
object Marks:
  var log = ""
  def mark[T](s: String, t: T): T = { log += s; t }
