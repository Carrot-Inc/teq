// Inline methods that tests/cases/inline_bodies_*.scala and tests/errors/inline_bodies_*.scala expand
// from scalac's pickles (the jar inlinebodies-lib): each body the definition typed, read back from
// TASTy and expanded as a program's own stored body is, so that the expansion is the one the same
// source gives (dotty's `Inlines.bodyToInline` over the `LazyBodyAnnotation` `TreeUnpickler` attaches).
package inlinebodies

import scala.compiletime.{codeOf, constValue, erasedValue}
import scala.deriving.Mirror
import scala.quoted.*

object Bodies:
  inline def value(x: Int): x.type = constValue[x.type]
  inline def code(x: Int): String = codeOf(x)
  inline def codeInline(inline x: Int): String = codeOf(x)
  inline def code2(inline x: Int): String = codeInline(x + 1)
  inline def proxied(x: Int): Boolean = codeOf(x).startsWith("x")
  inline def make(x: Int): () => Int =
    class C { def value: Int = x }
    val c = new C
    () => c.value
  inline def made(x: Int): String =
    class D(val y: Int) { override def toString = s"D($y,$x)" }
    new D(x + 1).toString
  inline def kind: Int = inline erasedValue[Int] match { case _: Int => 7 }
  inline def kindOf[T]: Int = inline erasedValue[T] match { case _: Int => 1; case _: String => 2; case _ => 3 }
  inline def zero[T]: T = erasedValue[T]

class Counter(start: Int):
  inline def plus(n: Int): Int = start + Bodies.value(n)

type Id[t] = t
type Lift0[F[_], T] <: Tuple = T match
  case EmptyTuple => EmptyTuple
  case a *: b => F[a] *: Lift0[F, b]

object Sizes:
  inline def size[K, E <: Tuple]: Int = constValue[Tuple.Size[E]]

trait Kind[Mono[_]]:
  type LiftP[F[_], T] <: Tuple = Mono[T] match
    case Tuple => Lift0[F, Mono[T]]
  inline def count[F[_], T](using gen: Mirror.ProductOf[T]): Int = Sizes.size[Kind.this.type, LiftP[F, gen.MirroredElemTypes]]

object K0 extends Kind[Id]

trait Ctx

class Mk[S]:
  transparent inline def apply[T](inline lambda: Ctx ?=> S => T): Any = ${ Shapes.of[S, T]('{ (c: Ctx) ?=> lambda(using c) }) }

object Focus:
  def apply[S] = new Mk[S]

object Shapes:
  def of[S: Type, T: Type](lambda: Expr[Ctx ?=> S => T])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def shape(t: Term): String = t match
      case Inlined(_, _, b) => shape(b)
      case Block(List(DefDef(_, _, _, Some(body))), Closure(_, _)) => "Lambda(" + shape(body) + ")"
      case Typed(e, _) => shape(e)
      case Block(Nil, e) => shape(e)
      case Select(_, n) => "Select(" + n + ")"
      case Apply(f, _) => "Apply(" + shape(f) + ")"
      case _ => "other"
    Expr(shape(lambda.asTerm))

object Quoted:
  inline def value(x: Int, inline f: Int => Int): Int = ${ valueImpl('{ f(1) }, '{ constValue[x.type] }) }
  def valueImpl(f: Expr[Int], x: Expr[Int])(using Quotes): Expr[Int] = x
  inline def code(inline x: Int, inline f: Int => Int): String = ${ codeImpl('{ f(1) }, '{ codeOf(x) }) }
  def codeImpl(f: Expr[Int], x: Expr[String])(using Quotes): Expr[String] = x
  inline def shape(inline f: Int => Int): String = ${ shapeImpl('{ f(1) }) }
  def shapeImpl(f: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(f.asTerm.show)
