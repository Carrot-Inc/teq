package muo

import scala.quoted.*

// An object's overloaded `apply`, one alternative of a using clause whose next parameter is a reflection tree
// (scala-library's abstract type `q.reflect.Term`, which its signature erases to Object), called with its using clause
// in a macro's body; and a sort, whose scala-library body calls the parent's `sorted` through a super accessor.
class Pc(val n: Int)
object Pc:
  def apply(n: Int): Pc = new Pc(n)
  def apply(using q: Quotes)(t: q.reflect.Term): Pc = new Pc(t.show.length)

object Mac:
  inline def width: Int = ${ widthImpl }
  def widthImpl(using q: Quotes): Expr[Int] =
    import q.reflect.*
    val pc = Pc(using q)(Literal(IntConstant(12345)))
    val least = List(3, 1, 2).sorted.head
    Expr(pc.n * 10 + least)
