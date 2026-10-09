package mca

import scala.quoted.*

object Impl:
  def code(using Quotes): Expr[Int] = Expr(Helper.value)
