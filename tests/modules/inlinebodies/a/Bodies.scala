// An upstream's inline bodies, expanded downstream from their pickles as the definition typed them
// (dotty's `Inlines.bodyToInline` over `TreeUnpickler`'s `LazyBodyAnnotation`): a constant argument
// is the parameter's path, `codeOf` shows the typed argument, each expansion's local class is a
// class of its own, an ascribed constant binds the proxy.
package iba

import scala.compiletime.{codeOf, constValue, erasedValue}

object Bodies:
  inline def value(x: Int): x.type = constValue[x.type]
  inline def both(x: Int): String = constValue[x.type].toString + "/" + x.toString
  inline def code(x: Int): String = codeOf(x)
  inline def make(x: Int): () => Int =
    class C { def value: Int = x }
    val c = new C
    () => c.value
  inline def kindOf[T]: Int = inline erasedValue[T] match { case _: Int => 1; case _ => 2 }
  inline def known(n: Int): Boolean = inline erasedValue[n.type] match { case _: 1 => true; case _ => false }

trait Der:
  def join(x: Int): String

// A class the body makes reads a member of the method's enclosing trait: the copy reads it on the
// receiver (`Inliner.computeThisBindings`) and is the member's function wherever a call dispatches.
trait Tagged:
  def tag: String = "smd"
  inline def derived(x: Int): String =
    class D extends Der:
      def join(x: Int): String = if x > 100 then "big" + prod(x) else prod(x)
      private def prod(x: Int): String = "p" + x + tag
    new D().join(x)

object Tag extends Tagged

// A local class with secondary constructors, which the copy's constructions go through.
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

// A type pattern over an abstract type the scrutinee has: a null test where the type filled in may
// be null.
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

// An inline body's test of its own type parameter, which a caller's type parameter fills: the erased
// test, everything but `null` (magnolia's default check).
object Abstract:
  inline def check[P](d: Any): Boolean = (d: @unchecked).isInstanceOf[P]
  inline def lazyCheck[P](d: Any): () => Option[P] = () =>
    val v: Any = d
    if (v: @unchecked).isInstanceOf[P] then Some(v.asInstanceOf[P]) else None
