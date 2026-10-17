// teq: --werror --target jvm --std=scala-library
// A plain ascription is the type written (TypeAssigner.scala 413), unannotated whatever else the
// program declares: `TypeComparer.lub` of it and the annotated ascription of the same type keeps
// the first, so the binding is checked, an unused method of an annotated result beside it,
// over the scala-library std.
import scala.annotation.internal.RuntimeChecked
object Main:
 def unused(): Option[Int] @RuntimeChecked = None
 def main(args: Array[String]): Unit =
  val a = Option(7)
  val Some(n) = if args.length == 0 then (a: Option[Int]) else (a: Option[Int] @RuntimeChecked)
  println(n)
// expect: 11:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
