// jars: scala-library cats-kernel cats-core
// Type class instances of cats resolved by summon; the syntax needs Scala 2 implicits and
// implicit conversions in the source language.
import cats.{Functor, Monoid, Show, Eq}
object Main:
  def main(args: Array[String]): Unit =
    val m: Monoid[Int] = summon[Monoid[Int]]
    println(m.combine(1, 2))
    val f: Functor[List] = summon[Functor[List]]
    println(f.map(List(1, 2))(_ + 1))
    val s: Show[Int] = summon[Show[Int]]
    println(s.show(1))
    val e: Eq[String] = summon[Eq[String]]
    println(e.eqv("a", "a"))
    println(Monoid[Int].empty)
    println(Functor[List].map(List(1, 2))(_ * 2))
