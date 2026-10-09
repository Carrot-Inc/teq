import scala.quoted.*

// Each object's `wide` is 1 where one of its `touch`es expanded before its first `wide`, else 0.
object A:
  private var wideSeen = false
  private var reversed = false
  inline def wide: Int = ${ wideImpl }
  inline def touch: Unit = ${ touchImpl }
  inline def one: 1 = { touch; 1 }
  def wideImpl(using Quotes): Expr[Int] =
    wideSeen = true
    Expr(if reversed then 1 else 0)
  def touchImpl(using Quotes): Expr[Unit] =
    if !wideSeen then reversed = true
    '{ () }

object B:
  private var wideSeen = false
  private var reversed = false
  inline def wide: Int = ${ wideImpl }
  inline def touch: Unit = ${ touchImpl }
  inline def one: 1 = { touch; 1 }
  def wideImpl(using Quotes): Expr[Int] =
    wideSeen = true
    Expr(if reversed then 1 else 0)
  def touchImpl(using Quotes): Expr[Unit] =
    if !wideSeen then reversed = true
    '{ () }

object C:
  private var wideSeen = false
  private var reversed = false
  inline def wide: Int = ${ wideImpl }
  inline def touch: Unit = ${ touchImpl }
  inline def one: 1 = { touch; 1 }
  inline def side: 1 = { touch; println("side"); 1 }
  def wideImpl(using Quotes): Expr[Int] =
    wideSeen = true
    Expr(if reversed then 1 else 0)
  def touchImpl(using Quotes): Expr[Unit] =
    if !wideSeen then reversed = true
    '{ () }
