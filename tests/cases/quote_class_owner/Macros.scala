import scala.quoted.*

trait IntCell:
  def read: Int

object CellFactory:
  inline def make(inline value: Int): IntCell = ${ makeImpl('value) }
  private def makeImpl(value: Expr[Int])(using Quotes): Expr[IntCell] =
    '{ new IntCell { def read: Int = $value } }
