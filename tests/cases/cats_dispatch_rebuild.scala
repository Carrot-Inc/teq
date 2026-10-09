// jars: scala-library cats-kernel cats-core
// A library class's method overriding a root whose name the reach pass suffixes after the final
// naming passes (cats' `Reducible.isEmpty` over `Foldable.isEmpty`): a rebuild names it as a
// fresh build does.
import cats.*
import cats.data.*
import cats.syntax.all.*

@main def run(): Unit =
  println("x0")
  println(NonEmptyList.of(1, 2).isEmpty)
  println(Foldable[List].isEmpty(List(1)))
  println(Foldable[List].isEmpty(Nil))
  println(Reducible[NonEmptyList].isEmpty(NonEmptyList.of(1)))
