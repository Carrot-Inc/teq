//> using dep org.typelevel::cats-core::2.13.0
import cats.data.NonEmptyList
import cats.syntax.all.*
object Main:
  def main(args: Array[String]): Unit =
    val n = NonEmptyList.of(1, 2, 3)
    println(n.map(_ * 2).toList)
    println(n.head + n.tail.sum)
    println(List(1, 2).toNel)
    println(NonEmptyList.fromList(List.empty[Int]))
