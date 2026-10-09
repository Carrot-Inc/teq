// A call of an abstract inline method on a parameter bound inside an expansion dispatches to
// the concrete inline implementation of the argument's class: `new Names` and the object
// `Lengths` (circe 0.14.16's `loopUnrolled` over its `Inliner`).
import scala.compiletime.erasedValue

sealed trait Inliner[A, Arg]:
  inline def apply[T](inline arg: Arg): A

class Names extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = scala.compiletime.constValue[T].toString

object Lengths extends Inliner[Int, String]:
  inline def apply[T](inline arg: String): Int = arg.length + scala.compiletime.constValue[T].toString.length

inline def loop[A, Arg, T <: Tuple](f: Inliner[A, Arg], inline arg: Arg): List[A] =
  inline erasedValue[T] match
    case _: EmptyTuple => Nil
    case _: (h *: ts) => f[h](arg) :: loop[A, Arg, ts](f, arg)

@main def run(): Unit =
  println(loop[String, Unit, ("a", "bb", "ccc")](new Names, ()))
  println(loop[Int, String, (1, 22)](Lengths, "xy"))
