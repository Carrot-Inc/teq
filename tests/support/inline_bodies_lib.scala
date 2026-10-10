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
  inline def both(x: Int): String = constValue[x.type].toString + "/" + x.toString
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

trait Der:
  def join(x: Int): String

trait Tagged:
  def tag: String = "smd"
  inline def derived(x: Int): String =
    val d: Der = new Der:
      def join(x: Int): String = if x > 100 then "big" + prod(x) else prod(x)
      private def prod(x: Int): String = "p" + x + tag
    d.join(x)

object Tag extends Tagged

// A quote's application of an inline parameter reduced as a whole (`BetaReduce`): a curried one, an
// argument of a constant type, an effect bound once before the closure's body.
object Reduce:
  inline def curried(inline f: Int => Int => Int): String = ${ showImpl('{ f(1)(2) }) }
  inline def folded(inline f: Int => Int): String = ${ showImpl('{ f(1 + 2) }) }
  inline def runCurried(inline f: Int => Int => Int): Int = ${ passImpl('{ f(1)(2) }) }
  inline def runEffect(inline f: Int => Int): Int = ${ passImpl('{ f({ println("a"); 1 }) }) }
  def showImpl(e: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(e.asTerm.show)
  def passImpl(e: Expr[Int])(using Quotes): Expr[Int] = e

// A local class with a secondary constructor that reads the method's parameter.
object Ctors:
  inline def make(n: Int): Int =
    class C(val v: Int):
      def this() = this(n)
      def get: Int = v
    new C().get
  inline def make2(n: Int): String =
    class D(val v: Int, val w: String):
      def this(s: String) = this(n, s)
      def this() = this("d")
      def get: String = w + v
    new D().get + new D("e").get + new D(1, "f").get

// A type pattern over an abstract type the scrutinee has: a test that the scrutinee is not null
// wherever the type the expansion fills in may be null (`TypeTestsCasts`' `testNotNull`).
object Tests:
  inline def matches[A](a: A, ok: Boolean): Boolean = a match
    case _: A if ok => true
    case _ => false
  inline def tester[A](ok: Boolean): A => Boolean = (x: A) => x match
    case _: A if ok => true
    case _ => false

// Classes an inline body makes whose members take by-name parameters (magnolia's type classes, tapir's
// `Derivation`): each expansion's copy keeps them by-name, its reads applying the thunks callers pass.
case class Boxed[T](v: T)
trait Joins[F[_]]:
  def join[T](name: String, inner: => F[T]): F[T]
  def wrap[T](x: T): F[T]
  inline def derived[T](x: T): F[T] = join("n", wrap(x))
  def viaThunk[T](x: T): F[T] = join("t", wrap(x))
object ByName:
  inline def derive[T](x: T): Boxed[T] =
    class Lazy extends Joins[Boxed]:
      def join[T](name: String, inner: => Boxed[T]): Boxed[T] =
        lazy val l = inner
        Boxed(l.v)
      def wrap[T](x: T): Boxed[T] = Boxed(x)
    new Lazy().derived[T](x)
  inline def derive2[T](x: T): Boxed[T] =
    class Cached extends Joins[Boxed]:
      def join[T](name: String, inner: => Boxed[T]): Boxed[T] = cached(name)(inner)
      private def cached[T](key: String)(f: => Boxed[T]): Boxed[T] = f
      def wrap[T](x: T): Boxed[T] = Boxed(x)
    new Cached().viaThunk[T](x)
