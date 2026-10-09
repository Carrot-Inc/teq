import scala.quoted.*

// Each object's `wide` is 1 where its `one` expanded before its first `wide`, else 0.
object A:
  private var wideSeen = false
  private var reversed = false
  inline def wide: Int = ${ wideImpl }
  inline def one: 1 = ${ oneImpl }
  def wideImpl(using Quotes): Expr[Int] =
    wideSeen = true
    Expr(if reversed then 1 else 0)
  def oneImpl(using Quotes): Expr[1] =
    if !wideSeen then reversed = true
    Expr(1).asInstanceOf[Expr[1]]

object B:
  private var wideSeen = false
  private var reversed = false
  inline def wide: Int = ${ wideImpl }
  inline def one: 1 = ${ oneImpl }
  def wideImpl(using Quotes): Expr[Int] =
    wideSeen = true
    Expr(if reversed then 1 else 0)
  def oneImpl(using Quotes): Expr[1] =
    if !wideSeen then reversed = true
    Expr(1).asInstanceOf[Expr[1]]

// Each expansion counts one.
object C:
  private var n = 0
  inline def wide: Int = ${ wideImpl }
  inline def one: 1 = ${ oneImpl }
  def wideImpl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
  def oneImpl(using Quotes): Expr[1] =
    n += 1
    Expr(1).asInstanceOf[Expr[1]]

object T:
  transparent inline def twice(inline x: Int): Int = x + x
