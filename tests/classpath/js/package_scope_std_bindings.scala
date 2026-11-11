// jars: scala-library cats-kernel cats-core
import cats.syntax.all.*

@main def packageCollectionScope(): Unit =
  val values = scala.collection.immutable.HashSet(1, 2)
  println(values.toList.sorted.show)
