// A member that takes several arguments as one tuple wins over a same-named extension, as in
// scalac; the extension is used when the member's parameter cannot hold a tuple.

final class Bag(val n: Int):
  def any(a: Any): String = "member any " + a.toString
  def gen[A](a: A): String = "member gen " + a.toString
  def tup(p: (Int, Int)): String = "member tuple " + p.toString
  def uni(u: String | (String, Boolean)): String = "member union " + u.toString
  def int(a: Int): String = "member int " + a.toString
  def opt(o: Option[Int]): String = "member option " + o.toString

final class Buf[A](val items: List[A]):
  def +=(a: A): Buf[A] = Buf(items :+ a)

extension (b: Bag)
  def any(x: Int, y: Int): String = "extension " + (x + y).toString
  def gen(x: Int, y: Int): String = "extension " + (x + y).toString
  def tup(x: Int, y: Int): String = "extension " + (x + y).toString
  def uni(x: Int, y: Int, z: Int): String = "extension " + (x + y + z).toString
  def int(x: Int, y: Int): String = "extension " + (x + y).toString
  def opt(p: (Int, Int)): String = "extension tuple " + p.toString

extension [A](b: Buf[A])
  def +=(x: A, y: A): Buf[A] = Buf(b.items :+ x :+ y)

@main def main(): Unit =
  val b = Bag(1)
  println(b.any(1, 2))
  println(b.gen(1, 2))
  println(b.tup(1, 2))
  println(b.uni("a", true))
  println(b.uni(1, 2, 3))
  println(b.int(1, 2))
  println(b.opt(1, 2))
  println(b.any(1))
  println(b.int(1))
  println(b.tup((1, 2)))
  println(b.opt(Some(1)))
  println(b any (1, 2))
  println(b gen (1, 2))
  println(b tup (1, 2))
  println(b uni ("a", true))
  println((Buf[(Int, Int)](Nil) += (1, 2)).items)
  println((Buf[Any](Nil) += (1, 2)).items)
