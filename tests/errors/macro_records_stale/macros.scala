package stale

import scala.quoted.*

/** A macro that keeps the symbol of its first expansion's site for its second: the owner chain
  * is the expansion's own, and the symbol names an entry of it that is gone. */
object Keeper:
  var owner: Any = null

  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    if owner == null then owner = Symbol.spliceOwner
    Expr(owner.asInstanceOf[Symbol].owner.name)

inline def site: String = ${ Keeper.impl }
