package web

import scala.quoted.*

// The function value a macro makes, which Applied.scala applies.
object AppliedMacro:
  inline def make: Int => Int = ${ makeImpl }
  def makeImpl(using Quotes): Expr[Int => Int] = '{ (x: Int) => x + 1 }
