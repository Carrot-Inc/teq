// A mutable map's `map`, `collect` and `flatMap` are overloaded as scala-library's `MapOps` and
// `IterableOps` overload them: a function over the entries that gives pairs builds a map, any other
// an iterable, written as a lambda, a typed lambda, a function value or a for-comprehension's yield.
import scala.collection.mutable

@main def run(): Unit =
  val m: mutable.Map[Int, Int] = mutable.Map(1 -> 2)
  var n = 0
  println(m.map(kv => kv._1 + kv._2).toList)
  println(m.map((kv: (Int, Int)) => kv._1 + kv._2).toList)
  println(m.map((k, v) => k * v).toList)
  val f: ((Int, Int)) => Int = kv => kv._1 + kv._2
  println(m.map(f).toList)
  println((for (k, v) <- m yield k + v).toList)
  val pairs: mutable.Map[Int, String] = m.map(kv => { n += 1; (kv._1, kv._2.toString) })
  println(pairs)
  val scalars: Iterable[Int] = m.map(kv => { n += 1; kv._2 })
  println(scalars.toList)
  println(n)
  println(m.collect { case (k, v) if v > 1 => k + v }.toList)
  println(m.collect { case (k, v) => (v, k) })
  println(m.flatMap(kv => List(kv._1, kv._2)).toList)
  println(m.flatMap(kv => List(kv._2 -> kv._1)))
  val h = mutable.HashMap(3 -> 4)
  println(h.map(kv => kv._1 * kv._2).toList)
