// A constant `final val` that implements an abstract member is read through its getter, which
// answers the constant: `Memoize` leaves it no field (zio's `RuntimeFlag` cases).
trait Flag:
  def index: Int
  def mask: Int
  def label: String

case object Interruption extends Flag:
  final val index = 0
  final val mask = 1 << index
  final val label = "interruption"

object Yielding extends Flag:
  final val index = 3
  final val mask: 8 = 8
  final val label = "yielding"

class Dynamic(val index: Int) extends Flag:
  final val mask = 16
  final val label = "dynamic"

@main def main(): Unit =
  val flags: List[Flag] = List(Interruption, Yielding, Dynamic(4))
  println(flags.map(f => s"${f.label} ${f.index} ${f.mask}"))
  println(flags.map(_.mask).foldLeft(0)(_ | _))
  println(Interruption.mask + Yielding.mask)
