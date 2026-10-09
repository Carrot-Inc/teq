package mqa

import scala.quoted.*

// Macros of the upstream, which a downstream expands from their pickled bodies: a quote of a splice, a tuple
// built, a splice nested in a
// quote with its own `Quotes`, a type shown.
object Mac:
  inline def twice(inline n: Int): Int = ${ twiceImpl('n) }
  def twiceImpl(n: Expr[Int])(using Quotes): Expr[Int] = '{ $n * 2 }

  inline def pair(inline a: Int, inline b: String): (Int, String) = ${ pairImpl('a, 'b) }
  def pairImpl(a: Expr[Int], b: Expr[String])(using Quotes): Expr[(Int, String)] = '{ ($a, $b) }

  inline def nested(inline x: Int): Int = ${ nestedImpl('x) }
  def nestedImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ val y = $x; ${ tenfold('y) } + 1 }
  def tenfold(y: Expr[Int])(using Quotes): Expr[Int] = '{ $y * 10 }

  inline def typeName[T]: String = ${ typeNameImpl[T] }
  def typeNameImpl[T: Type](using Quotes): Expr[String] = Expr(Type.show[T])

  // Classes a quote makes, copied at each expansion: the copies are named by the outermost
  // site, the split build's as the whole build's (kinds 2 and 6).
  inline def runner(inline msg: String): Runnable = ${ runnerImpl('msg) }
  def runnerImpl(msg: Expr[String])(using Quotes): Expr[Runnable] =
    '{ new Runnable { def run(): Unit = println("run " + $msg) } }

  inline def boxed(inline n: Int): Int = ${ boxedImpl('n) }
  def boxedImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    '{ class Box(val v: Int) { def twice: Int = v * 2 }; new Box($n).twice + new Box(1).v }
