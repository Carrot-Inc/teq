abstract class Entry(val key: String):
  override def toString = s"Entry($key)"
  override def hashCode = key.length

abstract class Plain(val key: String)

abstract class Named:
  def toString: String

trait Shouting:
  override def toString = "SHOUT"

case class Tagged(tag: String) extends Entry(tag)
case class Free(tag: String, n: Int) extends Plain(tag)
case class Loud(tag: String) extends Entry(tag) with Shouting
case class Own(tag: String) extends Entry(tag):
  override def toString = "own " + tag
  override def equals(that: Any) = that.isInstanceOf[Own]
case class Declared(n: Int) extends Named
case object Single extends Entry("single")
case object Bare extends Plain("bare")

class Override(val x: Int) extends Plain("o"):
  override def equals(that: Any) = that match
    case o: Override => o.x == x
    case _ => false
  override def hashCode = x
class Deeper(x: Int, val y: Int) extends Override(x)

@main def run(): Unit =
  println(Tagged("a"))
  println(Tagged("abc").hashCode)
  println(Tagged("a") == Tagged("a"))
  println(Tagged("a") == Tagged("b"))
  println(Free("f", 1))
  println(Free("f", 1) == Free("f", 1))
  println(Free("f", 1).hashCode == Free("f", 1).hashCode)
  println(Loud("l"))
  println(Own("o"))
  println(Own("o") == Own("p"))
  println(Declared(3))
  println(Single)
  println(Bare)
  println(Deeper(1, 2) == Deeper(1, 3))
  println(Deeper(1, 2) == Override(1))
  println(Set[Override](Deeper(1, 2), Override(1), Override(2)).size)
  println(Free("f", 1).copy(n = 2))
  println(Free("f", 1).productPrefix)
