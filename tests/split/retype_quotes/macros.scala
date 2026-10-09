package quotes

import scala.quoted.*

trait Show:
  def show: String

/** A quote that makes a class: every expansion has a copy of its own. */
object Macros:
  def shownImpl(text: Expr[String])(using Quotes): Expr[Show] =
    '{ new Show { def show: String = $text + "!" } }

inline def shown(inline text: String): Show = ${ Macros.shownImpl('text) }
