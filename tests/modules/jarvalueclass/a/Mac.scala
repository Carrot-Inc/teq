package jva

import scala.quoted.*

// A macro whose run in its own module calls a value class of scala-library (`ArrayOps`), which
// the whole build converts for the run: a downstream calls it as scalac does all the same,
// through the companion (`ArrayOps$.MODULE$.lastOption$extension`).
object Mac:
  inline def lastWord(inline s: String): String = ${ lastWordImpl('s) }
  def lastWordImpl(s: Expr[String])(using Quotes): Expr[String] =
    Expr(Array("none", s.valueOrAbort).lastOption.getOrElse(""))
