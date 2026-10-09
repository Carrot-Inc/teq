class Meters(val v: Int) extends AnyVal:
  def +(that: Meters): Meters = new Meters(v + that.v)
  def show: String = s"${v}m"

implicit class Rich(private val x: Int) extends AnyVal:
  def twice: Int = x * 2
  def times(n: Int): Int = x * n

case class UserId(id: Long) extends AnyVal

class Label(s: String) extends AnyVal:
  override def toString: String = s"Label($s)"

class Tagged(val n: Int) extends AnyVal:
  override def equals(that: Any): Boolean = that match
    case t: Tagged => t.n % 10 == n % 10
    case _ => false
  override def hashCode: Int = n % 10

object Main:
  def describe(a: AnyVal): String = a match
    case i: Int => s"int $i"
    case b: Boolean => s"boolean $b"
    case m: Meters => s"meters ${m.show}"
    case other => s"other"

  def main(args: Array[String]): Unit =
    val a = new Meters(1)
    val b = new Meters(2)
    println((a + b).show)
    println(3.twice)
    println(4.times(5))
    println(UserId(7))
    println(UserId(7) == UserId(7))
    println(new Meters(3) == new Meters(3))
    println(new Meters(3) == new Meters(4))
    println(new Meters(3).hashCode == new Meters(3).hashCode)
    val m = Map(UserId(1) -> "one", UserId(2) -> "two")
    println(m(UserId(2)))
    val s = Set(new Meters(1), new Meters(1), new Meters(2))
    println(s.size)
    println(new Label("x"))
    println(new Tagged(12) == new Tagged(22))
    val v: AnyVal = 3
    val w: AnyVal = a
    val x: Any = 5
    println(x.isInstanceOf[AnyVal])
    println((true: Any).isInstanceOf[AnyVal])
    println(("s": Any).isInstanceOf[AnyVal])
    println((List(1): Any).isInstanceOf[AnyVal])
    println(describe(1))
    println(describe(true))
    println(describe(a))
    println(describe(2L))
    println(UserId(3).id)
    val ids: List[AnyVal] = List(1, 2.5, 'c', ())
    println(ids.size)
