// A for comprehension's trailing `map` by the version (`enablesBetterFors`, `DropForMap`): every `map` called before
// 3.8; from 3.8 one of the generator's own pattern left out where the receiver's type is the call's, a `case K`
// and its body `K` by name, `(x)` unwrapped, `{ x }` a block, a `map` typed `Any` kept; a typed identifier the
// function's parameter, a literal pattern at its constant type.
class Seq1[A](val xs: List[A]):
  def map[B](f: A => B): Seq1[B] = { println("map"); Seq1(xs.map(f)) }
  def withFilter(p: A => Boolean): Seq1[A] = Seq1(xs.filter(p))
  override def toString = xs.mkString("Seq1(", ", ", ")")
class AnyBox(val v: Int):
  def map(f: Int => Int): Any = { println("any map"); AnyBox(f(v)) }
val K = 1
@main def m =
  println(for case K <- Seq1(List(1, 2)) yield K)
  println(for x <- Seq1(List(1, 2)) yield { x })
  println(for x <- Seq1(List(1, 2)) yield (x))
  println((for x <- AnyBox(1) yield x).isInstanceOf[AnyBox])
  for x: Any <- List(7) do println(x)
  for 1 <- List[1](1) do println("one")
