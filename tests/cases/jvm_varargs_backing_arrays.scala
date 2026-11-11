//> using platform jvm
// SeqLiterals preserves the element type of the array beneath a repeated argument.
import scala.collection.immutable.ArraySeq

class Meter(val value: Int) extends AnyVal

object Main:
  def show[A](xs: A*): Unit =
    val array = xs.asInstanceOf[ArraySeq[A]].unsafeArray
    println(array.getClass.getComponentType.getName)
    println(xs.length)

  def generic[A](x: A): Unit = show(x)

  def main(args: Array[String]): Unit =
    show("a", "b")
    show(1, 2)
    show[Double](1, 2.5)
    show((), ())
    show(new Meter(3))
    show[String]()
    generic("x")
    val array = Array(1, 2)
    show(array*)
