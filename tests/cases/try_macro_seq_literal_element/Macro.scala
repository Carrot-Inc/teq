// A macro whose expansion is a `try` with a handler, for try_macro_seq_literal_element.
import scala.quoted.*
object M:
  inline def caught(x: Int): Int = ${ caughtImpl('x) }
  def caughtImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    '{ try { if $x > 100 then 0 else throw new RuntimeException("r") } catch { case e: RuntimeException => List(1).map(i => i + e.getMessage.length).sum } }
