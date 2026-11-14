package ledger

import scala.quoted.*

/** What the macro below keeps while it runs: a map of the std's `java.util`, whose members call the
  * extension methods of its store. */
object Seen:
  private val held = new java.util.HashMap[String, Int]
  def once(key: String): Int =
    if held.containsKey(key) then held.get(key)
    else
      held.put(key, key.length)
      key.length

object Tally:
  inline def lengthOf(inline label: String): Int = ${ lengthOfImpl('label) }
  def lengthOfImpl(label: Expr[String])(using Quotes): Expr[Int] =
    Expr(Seen.once(label.valueOrAbort))
