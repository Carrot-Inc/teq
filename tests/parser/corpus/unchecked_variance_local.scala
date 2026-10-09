// `@uncheckedVariance` in a body: the registration of the annotated type, which the body's worker
// makes, is published with the class's check in the base's types.
import scala.annotation.unchecked.uncheckedVariance

object Test:
  def widen[A](xs: List[A]): Seq[A] =
    val ys: List[A @uncheckedVariance] = xs.reverse
    ys ++ xs

  def main(args: Array[String]): Unit =
    println(widen(List(1, 2, 3)))
