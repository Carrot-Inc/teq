// jars: scala-library cats-kernel cats-core
// `traverseFilter` over a List reaches `cats.instances.ListInstancesBinCompat0.
// catsStdTraverseFilterForList`, whose body teq refuses with `too many type arguments`, under
// both targets. scalac prints `Some(List(1, 3))`.
import cats.syntax.all.*
object Main:
  def main(args: Array[String]): Unit =
    println(List(1, 2, 3).traverseFilter(x => Option(Option(x).filter(_ % 2 == 1))))
