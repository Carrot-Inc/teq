/*
rules = ["file:../../support/DumpTokens.scala"]
*/
// The bridge's token contract (docs/FIX.md, "Token"; fix-speed's matrix): every token's kind, offsets, text, `is`,
// `isKeyword`, `isTrivia` and value, the lints all at `marker`.
package tokens

import scala.quoted.*

/* a block /* nested */ comment */
object TokenContract:
  val marker = 0 // assert: DumpTokens.token
  val x = 1; val c = 'c'
  val s = s"a${x}b$x$$" + raw"""q $x""" + s"" + "😀é"
  val n = 3L + 1.5f + 2d + 0x1F + 1e3
  val `back quoted` = 1
  type T = [A] =>> List[A]
  class C { type Foo = Int }
  type U = C#Foo
  def f(using y: Int)(implicit z: Int): Int ?=> Int = null.asInstanceOf[Int]
  inline def g: Int = ${ gImpl }
  def gImpl(using Quotes): Expr[Int] = '{ 1 }
  def h(e: Expr[Int])(using Quotes): Expr[Int] = '{ ${ e } + 1 }
  def q(using Quotes): Expr[Any] = '{ ${ 'x } }
  def l(using Quotes): Type[Int] = '[Int]
  for a <- List(1) if a > 0 yield a
  while x < 0 do ()
  end while
  if true then () else ()
  end if
  x match
    case _: Int => ()
  end match
  val t = 1	// a tab before
  extension (a: Int) def plus = a
  opaque type O = Int
  @deprecated val d = this
  class V[+A, -B](a: => A, bs: B*) { def u: Unit = return }
  try () catch { case e => throw e } finally ()
  given Int = 1
  val u = x.isInstanceOf[Matchable] && true || false
end TokenContract
