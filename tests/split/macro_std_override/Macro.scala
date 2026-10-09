package mso

import scala.quoted.*

inline def result(inline b: Boolean): Boolean = ${ resultImpl('b) }

// The `true` path reaches the std's sorted map and the `remove` of its descending map's values,
// an override of `Collection.remove`, which the output names apart from `List.remove(Int)`.
def resultImpl(b: Expr[Boolean])(using Quotes): Expr[Boolean] =
  if !b.valueOrAbort then Expr(false)
  else
    val m = new java.util.TreeMap[String, String]()
    m.put("key", "value")
    Expr(m.descendingMap().values().remove("value"))
