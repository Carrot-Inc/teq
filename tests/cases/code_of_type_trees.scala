// codeOf prints a written type as its typed tree (RefinedPrinter's toTextCore): a literal type as the literal, a
// singleton type as its path's `.type`, a type lambda with its parameters' bounds, a package path as written, and a
// class a `scala` package alias names (`Vector`, `BigInt`) by that name, an annotated type with its annotation.
import scala.compiletime.codeOf
import scala.annotation.unchecked.uncheckedVariance
object Obj
trait Functor[F[_]]
class tag(a: String = "d", b: Int = 0) extends scala.annotation.StaticAnnotation
class targ[T] extends scala.annotation.StaticAnnotation
class two(a: Int)(b: Int) extends scala.annotation.StaticAnnotation
class va(xs: Int*) extends scala.annotation.StaticAnnotation
class ti[T](x: T) extends scala.annotation.StaticAnnotation
class bn(x: => Int) extends scala.annotation.StaticAnnotation
class K:
  def here = codeOf(List.empty[this.type])

@main def run(): Unit =
  println(codeOf(List.empty[1]))
  println(codeOf(List.empty["s"]))
  println(codeOf(List.empty[1 | 2]))
  println(codeOf(List.empty[Option[1]]))
  val v = "x"
  println(codeOf(List.empty[v.type]))
  println(codeOf(List.empty[Obj.type]))
  println(K().here)
  println(codeOf(List.empty[Functor[[X] =>> List[X]]]))
  println(codeOf(List.empty[Vector[Int]]))
  println(codeOf(List.empty[BigInt]))
  println(codeOf(List.empty[Either[Int, String]]))
  println(codeOf(List.empty[Iterator[Int]]))
  println(codeOf(List.empty[collection.Seq[Int]]))
  println(codeOf(List.empty[scala.util.Try[Int]]))
  println(codeOf(List.empty[Int => String]))
  println(codeOf(List.empty[Int @unchecked]))
  println(codeOf(List.empty[List[Int] @unchecked]))
  println(codeOf(List.empty[Int @uncheckedVariance]))
  println(codeOf(List.empty[Int @tag("x")]))
  println(codeOf((1: Int @unchecked)))
  println(codeOf(List.empty[Option[Int @unchecked]]))
  println(codeOf(List.empty[Int @tag(b = 2)]))
  println(codeOf(List.empty[Int @tag()]))
  println(codeOf(List.empty[Int @targ[String]]))
  println(codeOf(List.empty[Int @two(1)(2)]))
  println(codeOf(List.empty[Int /* c */ @unchecked]))
  println(codeOf(List.empty[(Int) @unchecked]))
  println(codeOf(List.empty[Int /* see @deprecated */ @unchecked]))
  println(codeOf(List.empty[Int // see @deprecated
    @unchecked]))
  println(codeOf(List.empty[Int @va(1, 2, 3)]))
  println(codeOf(List.empty[Int @va()]))
  println(codeOf(List.empty[Int @ti(1)]))
  println(codeOf(List.empty[Int @ti[Long](1)]))
  println(codeOf(List.empty[Int @bn(1 + 1)]))
