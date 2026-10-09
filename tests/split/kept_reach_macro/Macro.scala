package km

import scala.quoted.*

inline def letters(inline s: String): Int = ${ lettersImpl('s) }

// `toList` and `count` run only for an argument with a `!`, as a macro's error path would.
def lettersImpl(s: Expr[String])(using Quotes): Expr[Int] =
  val text = s.valueOrAbort
  Expr(if text.contains("!") then text.toList.count(_.isLetter) else text.length)
