// The proxy of a by-value parameter takes the argument's own type, so an abstract inline
// member called on it reaches the class that implements it: an object, a class with a `using`
// clause or a type argument, a stable path, a local, a block's result, an `if` of one class in
// both branches and an `inline if` folded to either class.
import scala.compiletime.{erasedValue, constValue}

sealed trait Inliner[A, Arg]:
  inline def apply[T](inline arg: Arg): A

class Names extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = constValue[T].toString

object Lengths extends Inliner[Int, String]:
  inline def apply[T](inline arg: String): Int = arg.length + constValue[T].toString.length

class Conf(val prefix: String)
class Prefixed(using c: Conf) extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = c.prefix + constValue[T].toString

class Tagged[A] extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = constValue[T].toString + "@" + compiletime.constValue[A].toString

class Upper extends Inliner[String, Unit]:
  inline def apply[T](inline arg: Unit): String = constValue[T].toString.toUpperCase

object Holder:
  val names: Names = new Names
  val asTrait: Inliner[String, Unit] = new Names

inline def loop[A, Arg, T <: Tuple](f: Inliner[A, Arg], inline arg: Arg): List[A] =
  inline erasedValue[T] match
    case _: EmptyTuple => Nil
    case _: (h *: ts) => f[h](arg) :: loop[A, Arg, ts](f, arg)

inline def viaInlineIf[T <: Tuple](inline flag: Boolean): List[String] =
  loop[String, Unit, T](inline if (flag) new Names else new Upper, ())

@main def run(): Unit =
  given Conf = Conf("p:")
  println(loop[String, Unit, ("a", "bb", "ccc")](new Names, ()))
  println(loop[Int, String, (1, 22)](Lengths, "xy"))
  println(loop[String, Unit, ("a", "bb")](new Prefixed, ()))
  println(loop[String, Unit, ("a", "bb")](new Tagged["z"], ()))
  println(loop[String, Unit, ("a", "bb")](Holder.names, ()))
  val local = new Names
  println(loop[String, Unit, ("a", "bb")](local, ()))
  println(loop[String, Unit, ("a", "bb")]({ println("side"); new Names }, ()))
  val b = true
  println(loop[String, Unit, ("a", "bb")](if (b) new Names else new Names, ()))
  println(viaInlineIf[("a", "bb")](true))
  println(viaInlineIf[("a", "bb")](false))
