// A tuple element expected to be a type variable that the expected result bounds is adapted
// into the bound, as an argument of `Tuple2.apply` is under scalac: `modify[Long]` of
// `(pairs.size, m)` takes the `Int` to `Long` inside the `Tuple2`.
final class Box[+T](val t: T)
final class Ref[A](private var a: A):
  def modify[B](f: A => (B, A)): Box[B] =
    val (b, na) = f(a)
    a = na
    Box(b)

def setMany(r: Ref[Map[Int, Int]], pairs: List[(Int, Int)]): Box[Long] =
  r.modify: map =>
    (pairs.size, map ++ pairs.toMap)
def deleted(r: Ref[Map[Int, Int]], keys: Set[Int]): Box[Long] =
  r.modify { map =>
    (map.keySet.intersect(keys).size, map.removedAll(keys))
  }
def total: Box[Double] = Ref(1.5f).modify(f => (f, f))

@main def run(): Unit =
  val r = Ref(Map(1 -> 1))
  println(setMany(r, List(2 -> 2, 3 -> 3)).t + 1L)
  println(deleted(r, Set(1, 2, 9)).t)
  println(total.t)
