// Inside an inline expansion `C.this` is the receiver: a parameter's declared type names the
// receiver's type member, an explicit type argument spelt through `this` conforms to it, and a
// call of an abstract inline member from an anonymous class reaches the receiver's definition.
import scala.compiletime.{erasedValue, summonFrom, summonInline}
import scala.deriving.Mirror

object SealedTrait:
  class Subtype[F[_], A, S](val name: String)

trait SealedDerivation:
  type Typeclass[T]
  protected inline def deriveSubtype[s](m: Mirror.Of[s]): Typeclass[s]
  protected inline def step[A, s](m: Mirror.SumOf[A], idx: Int): List[SealedTrait.Subtype[Typeclass, A, ?]] =
    summonFrom {
      case mm: Mirror.SumOf[`s`] =>
        go[A, mm.MirroredElemTypes](mm.asInstanceOf[m.type], 0, Nil)
      case _ =>
        val tc = new Function0[Typeclass[s]]:
          override def apply(): Typeclass[s] = summonFrom {
            case tc: Typeclass[`s`] => tc
            case _ => deriveSubtype(summonInline[Mirror.Of[s]])
          }
        List(new SealedTrait.Subtype[Typeclass, A, s]("s" + idx + ":" + tc().toString))
    }
  protected transparent inline def go[A, T <: Tuple](m: Mirror.SumOf[A], idx: Int, result: List[SealedTrait.Subtype[Typeclass, A, ?]]): List[SealedTrait.Subtype[Typeclass, A, ?]] =
    inline erasedValue[T] match
      case _: EmptyTuple => result.sortBy(_.name)
      case _: (s *: tail) =>
        val sub = step[A, s](m, idx)
        go[A, tail](m, idx + 1, sub.:::[SealedTrait.Subtype[SealedDerivation.this.Typeclass, A, ?]](result))

trait Common[TC[_]]:
  type Typeclass[T] = TC[T]

trait Derivation[TC[_]] extends Common[TC] with SealedDerivation:
  def join[A](name: String): Typeclass[A]
  inline def derived[A](using m: Mirror.Of[A]): Typeclass[A] = inline m match
    case s: Mirror.SumOf[A] => join[A]("sum:" + go[A, s.MirroredElemTypes](s, 0, Nil).map(_.name).mkString(","))
    case p: Mirror.ProductOf[A] => join[A]("product")
  protected override inline def deriveSubtype[s](m: Mirror.Of[s]): Typeclass[s] = derived[s](using m)

trait Enc[T]:
  def show: String
  override def toString: String = show
object DeriveEnc extends Derivation[Enc]:
  def join[A](name: String): Enc[A] = new Enc[A] { def show = name }

sealed trait Pet
case class Dog(b: Int) extends Pet
case class Cat(l: Int) extends Pet

object Main:
  def main(args: Array[String]): Unit =
    println(DeriveEnc.derived[Pet].show)
    println(DeriveEnc.derived[Dog].show)
