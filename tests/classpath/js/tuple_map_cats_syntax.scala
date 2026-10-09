// jars: cats-kernel cats-core
//> using dep org.typelevel::cats-core:2.13.0
// targets: js interp jvm
// `Tuple`'s `map` takes a polymorphic function; a pair's `map(f)` with a plain lambda does not
// type against it and gives way to cats' `Functor` syntax for `(A, *)`, a conversion of the
// receiver, as scalac's `tryInsertImplicitOnQualifier` has it (an application's
// `settings.map(_.map(_.toLowerCase))`), and to the program's own extension where one applies.
import cats.syntax.all.*

extension (p: (Int, Int)) def zip(f: Int => Int): Int = f(p._1) + f(p._2)

object Main:
  def main(args: Array[String]): Unit =
    val settings = List(("Colors", "AbC"), ("Fonts", "DeF"))
    println(settings.map(_.map(_.toLowerCase)))
    println(("k", 1).map(_ + 1))
    println((1, 2).zip(_ * 10))
    println((1, 2).map(identity))
    println((1, 2).map([t] => (x: t) => List(x)))
