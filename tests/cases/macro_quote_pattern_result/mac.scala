// A transparent macro whose result type comes from a quote pattern's type binder, through
// encoders combined in constructed quotes (skunk's `sql` fold, with the block a pickled quote
// holds and its using clause passed): the expansion's static type at the site is the bound
// type, not a wildcard, and a quote's block body has its result's type.
import scala.quoted.*
import Enc.*

trait Enc[A]:
  def sql: String
  def imap[B](f: A => B)(g: B => A): Enc[B] = Enc.of(sql)
object Enc extends TwiddleSyntax[Enc]:
  def of[A](s: String): Enc[A] = new Enc[A] { def sql = s }
  given Inv[Enc] with
    def imap[A, B](fa: Enc[A])(f: A => B)(g: B => A): Enc[B] = Enc.of(fa.sql)

trait Inv[F[_]]:
  def imap[A, B](fa: F[A])(f: A => B)(g: B => A): F[B]
final class TwiddleOpCons[F[_], B <: Tuple](private val fb: F[B]) extends AnyVal:
  def *:[A](fa: F[A])(using inv: Inv[F], ev: DummyImplicit): F[A *: B] =
    inv.imap(fa)(a => (a *: EmptyTuple).asInstanceOf[A *: B])(t => t.productElement(0).asInstanceOf[A])
  def *:[G[x] >: F[x], A](ga: G[A])(using inv: Inv[G]): G[A *: B] =
    inv.imap(ga)(a => (a *: EmptyTuple).asInstanceOf[A *: B])(t => t.productElement(0).asInstanceOf[A])
trait TwiddleSyntax[F[_]]:
  implicit def toTwiddleOpCons[B <: Tuple](fb: F[B]): TwiddleOpCons[F, B] = new TwiddleOpCons(fb)

final class Codec[A](val sql: String) extends Enc[A]
final case class Frag[A](sql: String, enc: Enc[A]):
  def show: String = sql + ":" + enc.sql
object Frag:
  def fromParts[A](s: String, e: Enc[A]): Frag[A] = Frag(s, e)

object Mac:
  def impl(argsExpr: Expr[Seq[Any]])(using Quotes): Expr[Any] =
    val Varargs(args) = argsExpr: @unchecked
    val encoders: List[Expr[Any]] = args.toList.map {
      case '{ $e: Enc[t] } => '{ $e : Enc[t] }
    }
    val finalEnc: Expr[Any] =
      if encoders.size == 1 then encoders.head
      else
        val last: Expr[Any] = encoders.last match
          case '{ $a: Enc[a] } => '{ $a.imap(_ *: EmptyTuple)(_.head) }
        encoders.init.foldRight(last) { case ('{ $a: Enc[a] }, '{ $acc: Enc[t & Tuple] }) =>
          '{ val fa1: Enc[a] = $a; Enc.toTwiddleOpCons[t & Tuple]($acc).*:(fa1)(using summon[Inv[Enc]], DummyImplicit.dummyImplicit) } }
    finalEnc match
      case '{ $e : Enc[t] } => '{ Frag.fromParts[t]("q", $e) }

  transparent inline def mk(inline args: Any*): Any = ${ impl('args) }
