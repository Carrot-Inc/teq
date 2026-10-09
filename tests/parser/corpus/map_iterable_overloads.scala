// `map`, `collect` and `flatMap` on a Map: a function giving pairs builds a Map, any other an
// Iterable, through the overloads of scala-library's MapOps and IterableOps.
object Main:
  def main(args: Array[String]): Unit =
    val m = Map("a" -> 1, "b" -> 2, "c" -> 3)
    println(m.map((k, v) => (k + k, v * 2)))
    println(m.map(_._1))
    println(m.map(kv => kv._2 + 1).sum)
    val vs: Iterable[Int] = m.map(_._2)
    println(vs)
    println(m.collect { case (k, v) if v > 1 => k -> v })
    println(m.collect { case (k, v) if v > 1 => k })
    println(m.flatMap((k, v) => List(k -> v, (k + "!") -> v)).toList.sorted)
    println(m.flatMap((k, v) => List(v, v)))
    val it: Iterable[(String, Int)] = m
    println(it.map(_._1))
    println(it.map(kv => (kv._1, kv._2 + 1)))
    println(m.map(kv => kv).getOrElse("a", 0))
    val names = Array("a", "b", "a")
    println(names.groupBy(identity).collect { case (n, ns) if ns.lengthCompare(1) > 0 => n }.mkString(","))
    println(m.map(_._1).toList.sorted)
