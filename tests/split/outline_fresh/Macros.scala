package lib

import scala.quoted.*

// A macro splicing its caller's lambda into the expansion, as tapir's `schemaForMap(keyToString)` does: the lambda's
// placeholder is a fresh name of the caller's file (`_$1` in one, `_$4` in another), the expansion's one difference.
object Keys:
  inline def keyed[K](keys: List[K], show: K => String): String = ${ keyedImpl('keys, 'show) }

  def keyedImpl[K: Type](keys: Expr[List[K]], show: Expr[K => String])(using Quotes): Expr[String] =
    '{
      val shown = $keys.map(k => $show(k))
      val joined = shown.mkString(",")
      if joined.isEmpty then "none" else "[" + joined + "]"
    }
