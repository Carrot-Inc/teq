package p

import scala.quoted.*

/* block /* nested */ */
object I:
  val x = 1; val c = 'c'
  val s = s"a${x}b$x$$"
  val r = raw"""q $x"""
  val e = s""
  val n = 3L + 1.5f + 2d + 0x1F + 1e3
  type T = [A] =>> List[A]
  class C { type Foo = Int }
  type U = C#Foo
  def f(using y: Int)(implicit z: Int): Int ?=> Int = null.asInstanceOf[Int]
  inline def g: Int = ${ gImpl }
  def gImpl(using Quotes): Expr[Int] = '{ 1 }
  def h(e: Expr[Int])(using Quotes): Expr[Int] = '{ ${ e } + 1 }
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
  val forSome = 2
  extension (a: Int) def plus = a
  opaque type O = Int
  val b = new Object
  @deprecated val d = this
  class V[+A, -B](a: => A, bs: B*) { def u: Unit = return }
  try () catch { case e => throw e } finally ()
  given Int = 1
  val u = x.isInstanceOf[Matchable] && true || false
  val crlf = 1
  val ff = 2
  val cr = 3  // lone
end I
