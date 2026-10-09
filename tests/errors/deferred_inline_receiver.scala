// expect: 27:56: error: Deferred inline method apply in trait Inliner cannot be invoked
// expect: 1 error found
// A call of an abstract inline method inside an expansion whose receiver's static type stays
// the trait is refused as scalac refuses it: a parameter or a value typed as the trait, an `if`
// of two different classes, an anonymous class (its type is the parent's). The first error
// stops the rest of its expansion, and the later calls fail at the same place of `loop`'s body,
// where scalac 3.8.4 reports one error alone (`UniqueMessagePositions`).
import scala.compiletime.{erasedValue, constValue}

sealed trait Inliner[A, Arg]:
  inline def apply[T](inline arg: Arg): A

class Names extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = constValue[T].toString

class Upper extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = constValue[T].toString.toUpperCase

object Holder:
  val asTrait: Inliner[String, Unit] = new Names

inline def loop[A, Arg, T <: Tuple](f: Inliner[A, Arg], inline arg: Arg): List[A] =
  inline erasedValue[T] match
    case _: EmptyTuple => Nil
    case _: (h *: ts) => f[h](arg) :: loop[A, Arg, ts](f, arg)

def viaParam(f: Inliner[String, Unit]): List[String] = loop[String, Unit, ("a", "bb")](f, ())

@main def run(): Unit =
  println(viaParam(new Names))
  println(loop[String, Unit, ("a", "bb")](Holder.asTrait, ()))
  val b = true
  println(loop[String, Unit, ("a", "bb")](if (b) new Names else new Upper, ()))
  val typed: Inliner[String, Unit] = new Names
  println(loop[String, Unit, ("a", "bb")](typed, ()))
  println(loop[String, Unit, ("a", "bb")](new Inliner[String, Unit] { inline def apply[T](inline arg: Unit): String = "anon" }, ()))
