class Box(val n: Int):
  def size: Int = n
object O:
  def f(g: Int => Int): Int = g(1)
  def f(g: String => Int, k: Int = 0): Int = g("a")
  def h[T](x: T)(g: T => Int): Int = g(x)
@main def run(): Unit =
  println(O.f((x: Int) => { extension (b: Box) def size: Int = 0; x }))
  println(O.f(x => { extension (b: Box) def size: Int = 0; 2 }))
  println(O.h(3)(x => { extension (b: Box) def size: Int = 0; x }))
  val ys = List(1, 2).map { y => extension (b: Box) def size: Int = 0; y + Box(y).size }
  println(ys)
