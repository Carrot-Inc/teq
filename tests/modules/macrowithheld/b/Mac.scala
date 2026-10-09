package mwb

import scala.quoted.*

object Mac:
  inline def count(): Int = ${ countImpl() }
  def countImpl()(using Quotes): Expr[Int] =
    Expr(mwa.Jobs.make.run())
