// A path's type conforms to a type parameter bounded below by it (`T >: h.Start` takes
// `h.start`), as a library body's pattern capture `Box[? >: h.Start]` needs.
class Box[T](val value: T):
  def put(x: T): String = "put(T) " + x

trait St:
  type Start
  def start: Start

class C(val h: St):
  def g[T >: h.Start](b: Box[T]): String = b.put(h.start)

def k(h: St)(b: Box[Any]): String =
  def inner[T >: h.Start](bb: Box[T]): String = bb.put(h.start)
  inner(b)

@main def main(): Unit =
  val s = new St { type Start = Int; def start = 7 }
  println(C(s).g(Box[Any](1)))
  println(k(s)(Box[Any](1)))
