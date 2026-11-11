package mih

import scala.quoted.*

object M:
  inline def k: Int = 3
  inline def m(inline x: Int): Int = x * 2
  inline def d(inline x: Int = 4): Int = x * 2
  transparent inline def tvia: Int = ${ tviaImpl }
  def tviaImpl(using Quotes): Expr[Int] = '{ M.m(M.k) + 1 }
  transparent inline def tdef: Int = ${ tdefImpl }
  def tdefImpl(using Quotes): Expr[Int] = '{ M.d(4) + M.m(M.d(4)) }
