package p

import scala.quoted.*

inline def answer: Int = ${ impl }

def impl(using Quotes): Expr[Int] =
  val n = summon[Source].read()
  if n != 42 then quotes.reflect.report.error("file value " + n)
  Expr(n)
