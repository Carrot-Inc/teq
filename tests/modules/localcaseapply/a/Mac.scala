package lca

import scala.quoted.*

object Mac:
  inline def label(inline s: String): String = ${ labelImpl('s) }

  def labelImpl(s: Expr[String])(using Quotes): Expr[String] =
    case class Part(text: String, count: Int)
    val parts = List(s.valueOrAbort, "cde").map(w => Part(w, w.length))
    Expr(parts.map(p => s"${p.text}:${p.count}").mkString(","))

  def describe(n: Int): String =
    case class Pair(left: Int, right: Int)
    val p = Pair(n, n * 2)
    s"${p.left}-${p.right}"
