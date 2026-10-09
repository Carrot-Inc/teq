// A class below java.lang.Number compares with the numeric primitives, as under scalac, and a
// Double case takes the Ints of a union, since both are one JS number.
final class Big(val v: Int) extends java.lang.Number:
  def intValue(): Int = v
  override def equals(o: Any): Boolean = o match
    case b: Big => b.v == v
    case i: Int => i == v
    case _ => false

def kind(x: Int | Long | Double): String = x match
  case l: Long => "long"
  case d: Double => "number"

@main def run(): Unit =
  println(new Big(5) == 5)
  println(new Big(5) == 6L)
  println(new Big(5) != 2.5)
  println(kind(5))
  println(kind(5L))
  println(kind(2.5))
