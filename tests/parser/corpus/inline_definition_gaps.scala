// teq: --inline-definition-errors
// The four gaps the definition check found on bodies scalac 3.8.4 accepts, each also a gap of
// plain code: a type variable of a type pattern takes the bound of the parameter it stands for
// (`t` of `h *: t` is a `Tuple`, `a` of `Box[a]` an `AnyVal`); a singleton instance widens where
// a type argument is inferred unless its bound is `Singleton` (`named(own(x))` is `named[B]`, and
// `x.type & B` a `B`); a type parameter bounded by `String` or a primitive has its bound's
// operators (`l + "="`, `n + 1`); `head` and `tail` of a tuple whose length is open
// (`tt: (h *: rest)`), and `h *: t` tested at run time as a tuple of one element or more.
import scala.compiletime.{constValue, erasedValue}

trait B
object O extends B
trait TC[A]:
  def name: String
given TC[B] with
  def name = "TC[B]"
def named[T](x: T)(using tc: TC[T]): String = tc.name

object Plain:
  def needTuple[T <: Tuple](x: T): Int = x.productArity
  def rest(x: Any): Int = x match
    case p: (h *: t) => needTuple[t](p.tail)
    case _ => -1
  class Box[A <: AnyVal](val a: A)
  def needVal[A <: AnyVal](a: A): String = a.toString
  def boxed(x: Any): String = x match
    case b: Box[a] => needVal[a](b.a)
    case _ => "none"
  def own(x: B): x.type = x
  def viaOwn(x: B): String = named(own(x))
  def both(x: B): x.type & B = x
  def viaBoth(x: B): String = named(both(x))
  def label[L <: String](l: L): String = l + "="
  def next[N <: Int](n: N): Int = n + 1
  def flip[F <: Boolean](f: F): Boolean = f ^ true
  def parts[T <: Tuple](t: T): String = t match
    case tt: (h *: rest) => tt.head.toString + "|" + tt.tail.toString
    case _ => "empty"
  def nonEmpty(x: Any): Boolean = x match
    case _: (h *: t) => true
    case _ => false

object Inline:
  inline def names[T <: Tuple]: List[String] = inline erasedValue[T] match
    case _: EmptyTuple => Nil
    case _: (Int *: ts) => "Int" :: names[ts]
    case _: (t *: ts) => "other" :: names[ts]
  transparent inline def ownT(x: B): x.type = x
  inline def viaOwn(x: B): String = named(ownT(x))
  inline def describe[L <: String]: String =
    val label = constValue[L]
    label + "=" + label.length
  inline def sumInts[T <: Tuple](t: T): Int =
    inline t match
      case EmptyTuple => 0
      case tt: (h *: rest) =>
        inline tt.head match
          case i: Int => i + sumInts(tt.tail)
          case _ => sumInts(tt.tail)

@main def run(): Unit =
  println(Plain.rest((1, "a", true)))
  println(Plain.boxed(Plain.Box(3)))
  println(Plain.viaOwn(O) + " " + Plain.viaBoth(O))
  println(s"${Plain.label("a")} ${Plain.next(3)} ${Plain.flip(true)}")
  println(Plain.parts((1, "x", true)) + " " + Plain.parts(EmptyTuple))
  println(s"${Plain.nonEmpty((1, 2))} ${Plain.nonEmpty(1)} ${Plain.nonEmpty(EmptyTuple)}")
  println(Inline.names[(Int, String, Int)])
  println(Inline.viaOwn(O))
  println(Inline.describe["key"])
  println(Inline.sumInts((1, "a", 2, 3.0, 4)))
