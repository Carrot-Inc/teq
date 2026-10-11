// A method eta-expanded without an expected type keeps its by-name parameters by-name
// (`EtaExpansion.etaExpand` declares the method's own parameter types): `twice` is a
// `(=> Int) => Int` that evaluates its argument at each use; expected as an `Int => Int` it is not.
// Two such functions join in one that takes its argument by name (dotty's `glb` of `ExprType`s).
def twice(x: => Int): Int = x + x
def byName[T](p: => T): T = p
def both(x: => Int, y: Int): Int = x + y + x
inline def itwice(x: => Int): Int = x + x
class Box:
  def m(x: => String)(y: => String): String = x + y + x
@main def run(): Unit =
  var n = 0
  val f = twice
  println(f({ n += 1; n }))
  println(n)
  val g = both
  println(g({ n += 1; n }, 100))
  println(n)
  val b = new Box
  val h = b.m
  println(h({ n += 1; n.toString })({ n += 10; n.toString }))
  val k: Int => Int = twice
  println(k({ n += 1; n }))
  println(n)
  val i = itwice
  println(i({ n += 1; n }))
  println(n)
  val any: Any = twice
  println(any.isInstanceOf[Function1[?, ?]])
  var m = 0
  val joined = if m == 0 then byName else (??? : ((=> Int) => Int))
  println(joined({ m += 1; m }))
  val either = if m > 5 then byName else twice
  println(either({ m += 1; m }))
  println(m)
