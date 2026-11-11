package mck

import scala.quoted.*

// Overloads whose using alternative a macro's implementation calls: one taking a reflection trait (scala-library's
// `Quotes#reflectModule.Printer`, a class in its signature, where a tree erases to Object), and one a trait's
// override calls through `super`, beside an ordinary overload of its name.
object Pc:
  def apply(n: Int): Int = n
  def apply(using q: Quotes)(t: q.reflect.Printer[q.reflect.Tree]): Int = 42

trait Base:
  def f(n: Int): Int = n + 1
  def f(using s: String)(n: Int): Int = n + s.length
trait Derived extends Base:
  override def f(using s: String)(n: Int): Int = super.f(using s)(n) + 10
class Impl extends Derived

// Nested: a top-level object's static forwarders come out of scalac's two Scala.js builds in another order.
object Macros:
  object Mac:
    inline def printer: Int = ${ printerImpl }
    def printerImpl(using q: Quotes): Expr[Int] =
      import q.reflect.*
      Expr(Pc(using q)(Printer.TreeCode))
    inline def sup: Int = ${ supImpl }
    def supImpl(using Quotes): Expr[Int] = Expr(new Impl().f(using "abc")(7))
