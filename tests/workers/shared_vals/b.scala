package macros

import scala.quoted.*

inline def answer: Int = ${ impl }

def impl(using Quotes): Expr[Int] =
  val n = data.observed
  if n != 42 then quotes.reflect.report.error("observed " + n)
  Expr(n)
