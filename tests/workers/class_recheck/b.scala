package p

import scala.quoted.*

inline def answer: Int = ${ impl }

def impl(using Quotes): Expr[Int] =
  val n = summon[Factory].make().value
  if n != 42 then quotes.reflect.report.error("class value " + n)
  Expr(n)

inline def noop: Unit = ${ noopImpl }
def noopImpl(using Quotes): Expr[Unit] = '{ () }
