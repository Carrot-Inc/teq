// jars: scala-library cats-kernel cats-core
// cats.data.Chain against the real jars: its iterators extend the std's Iterator, its
// constructors and `Append.apply` are overloaded, and `drop` binds wildcards with equal bounds.
//> using dep org.typelevel::cats-core:2.13.0
import cats.data.Chain

object Main:
  def main(args: Array[String]): Unit =
    val c = Chain(1, 2) ++ Chain(3) ++ Chain.one(4)
    println(c.toList)
    println(c.map(_ * 2).filter(_ > 2).toList)
    println(c.foldLeft(0)(_ + _))
    println(c.drop(1).toList)
    println(c.headOption)
    println(c.length)
    println(c.iterator.toList)
    println(Chain.empty[Int].isEmpty)
