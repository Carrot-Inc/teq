package mss

import scala.quoted.*

// Super accessors a macro's implementation reaches, of a member beside an ordinary overload of its name: one whose
// using parameter's type is a type parameter the subclass instantiates, one polymorphic.
trait Base[A]:
  def f(n: Int): Int = n + 1
  def f(using a: A)(n: Int): Int = n + 3
trait Derived extends Base[String]:
  def g(n: Int): Int = super.f(using "abc")(n) + 10
class Impl extends Derived

trait PolyBase:
  def f(n: Int): Int = n + 1
  def f[A](using a: A)(n: Int): Int = n + 3
trait PolyDerived extends PolyBase:
  def g(n: Int): Int = super.f[String](using "abc")(n) + 10
class PolyImpl extends PolyDerived

// Nested: a top-level object's static forwarders come out of scalac's two Scala.js builds in another order.
object Macros:
  object Mac:
    inline def seen: Int = ${ seenImpl }
    def seenImpl(using Quotes): Expr[Int] = Expr(new Impl().g(7))
    inline def poly: Int = ${ polyImpl }
    def polyImpl(using Quotes): Expr[Int] = Expr(new PolyImpl().g(7))
