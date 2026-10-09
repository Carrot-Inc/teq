package state

import scala.quoted.*

/** A macro with a state of its own, the number of its expansions: the program calls it once,
  * and it fails when it finds that it has expanded before. */
object Expansions:
  var seen: Int = 0

  def onceImpl(text: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    seen += 1
    if seen > 1 then report.errorAndAbort(s"expanded $seen times", text)
    text

inline def once(inline text: String): String = ${ Expansions.onceImpl('text) }
