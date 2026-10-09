package shown

import scala.quoted.*

/** A macro that keeps its first expansion's owner, a lambda of the site, and renders it in its
  * second: rendering reads the entry of the owner chain as much as a call on the symbol does. */
object Keeper:
  var saved: Any = null

  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    if saved == null then saved = Symbol.spliceOwner.owner
    Expr(saved.toString)

inline def shown: String = ${ Keeper.impl }
