// An extension named like an assignment operator only takes over for receivers it applies to;
// `i += 1` on a var of another type stays an assignment.

final class Buf[A](val items: List[A]):
  override def toString: String = "Buf" + items.toString

extension [A](b: Buf[A])
  def +=(x: A): Buf[A] = Buf(b.items :+ x)
  def ++=(x: A, y: A): Buf[A] = Buf(b.items :+ x :+ y)
  def -=(x: A): Buf[A] = Buf(b.items.filter(_ != x))

extension (s: String)
  def *=(n: Int): String = List.fill(n)(s).mkString("")

@main def main(): Unit =
  var i = 0
  i += 1
  i -= 5
  i *= 3
  println(i)
  var d = 1.5
  d += 1
  println(d)
  var text = "a"
  text += "b"
  println(text)
  println(text *= 2)
  var xs = List(1)
  xs :+= 2
  xs ++= List(3)
  println(xs)
  var pairs = List((1, "a"))
  pairs :+= (2, "b")
  println(pairs)
  var buf = Buf[Int](Nil)
  println(buf += 1)
  println(buf ++= (1, 2))
  println((buf += 1) -= 1)
  buf = buf += 7
  println(buf)
  val pb = Buf[(Int, Int)](Nil)
  println(pb += (1, 2))
  println(pb ++= ((1, 2), (3, 4)))
