package mta

import scala.quoted.*

// Transparent macros of the upstream, whose expansions the middle module's pickles hold
// (`INLINED`, same as scalac's): a downstream reading them names the classes a quote made from
// the records, as the whole build names them (kinds 2 and 8 of the TeqOrigins).
object T:
  transparent inline def pick(inline b: Boolean): Any = ${ pickImpl('b) }
  def pickImpl(b: Expr[Boolean])(using Quotes): Expr[Any] =
    if b.valueOrAbort then '{ 1 } else '{ "one" }

  transparent inline def runner(inline msg: String): Runnable = ${ runnerImpl('msg) }
  def runnerImpl(msg: Expr[String])(using Quotes): Expr[Runnable] =
    '{ new Runnable { def run(): Unit = println("run " + $msg) } }
