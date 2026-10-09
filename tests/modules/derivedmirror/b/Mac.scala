package dmb2

import scala.quoted.*
import dma2.*

object Mac:
  inline def made: String = ${ madeImpl }
  def madeImpl(using Quotes): Expr[String] =
    Expr(summon[Schema[Email]].make(Tuple1("in a macro")).toString + " " + summon[Schema[Point]].make((1, 2)))
