// A generator's function is a function literal (dotty's `Desugar.makeFor`) and is typed ahead of
// the choice among an overloaded method's alternatives as a written one is
// (`Applications.pretypeArgs`): over a `Map`, a yield of no pair is an `Iterable` of the values,
// a yield of pairs a `Map`.
import scala.collection.immutable.SortedMap

@main def run(): Unit =
  val m = Map(1 -> 2, 3 -> 4)
  println((for (x, y) <- m yield x + y).mkString(","))
  println((for case (x, y) <- m yield x + y).mkString(","))
  println((for kv <- m yield kv._1 + kv._2).mkString(","))
  println(for (x, y) <- m yield (x + 1, y + 1))
  println((for (k, v) <- m if k > 1 yield k + v).mkString(","))
  println((for (k, v) <- m; s = k + v yield s).mkString(","))
  println((for (k, v) <- m; n <- List(k, v) yield n * 10).mkString(","))
  println((for (k, v) <- m; (p, q) <- Map(k -> v) yield (p, q + 1)).mkString(","))
  println(for x <- List(1, 2); (k, v) <- Map(x -> 10) yield k + v)
  for (k, v) <- m do println(k * v)
  val sorted = SortedMap(2 -> "b", 1 -> "a")
  println((for (k, v) <- sorted yield v * k).mkString(","))
  println(for (k, v) <- sorted yield (v, k))
  val xs: List[Any] = List((1, 2), 3)
  println((for case (x, y) <- xs yield x).mkString(","))
