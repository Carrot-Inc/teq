// A stored inline body keeps the type arguments of a call an implicit conversion built
// (`box[Int](x)` here): the census checks it.
import scala.language.implicitConversions
case class Box[A](a: A)
implicit def box[A](a: A): Box[A] = Box(a)
def unbox[A](b: Box[A]): A = b.a
inline def f(x: Int): Int = unbox(x)
@main def run(): Unit = println(f(3))
