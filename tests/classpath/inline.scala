// jars: scala-library
// Checks with no error: the inline methods of scala-library expand from their TASTy bodies,
// and scala.compiletime comes from the jar.
import scala.compiletime.{constValue, erasedValue, summonInline}
import scala.compiletime.ops.int.+

trait Codec[A]:
  def name: String

object Codec:
  given Codec[Int] with
    def name: String = "int"
  given Codec[String] with
    def name: String = "string"

object Main:
  def notNull(s: String | Null): String = s.nn
  def same(a: AnyRef, b: AnyRef): Boolean = (a eq b) && !(a ne b)
  def local: Int = locally { val k = 2; k + 1 }
  def checked(n: Int): Int =
    assert(n > 0, "positive")
    assert(n < 100)
    n
  def summoned: Codec[Int] = summon[Codec[Int]]

  inline def codecNames[T <: Tuple]: List[String] = inline erasedValue[T] match
    case _: EmptyTuple => Nil
    case _: (t *: ts) => summonInline[Codec[t]].name :: codecNames[ts]

  inline def width[N <: Int]: Int = constValue[N]

  val names: List[String] = codecNames[(Int, String)]
  val three: 1 + 2 = 3
  val w: Int = width[8]
  val pair: (Int, String) = 1 *: "a" *: EmptyTuple
