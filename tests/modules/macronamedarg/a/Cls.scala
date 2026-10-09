package mna

import scala.quoted.*

// Macros called as named arguments after a defaulted one, which the call's block binds: their
// `NAMEDARG` and reference stand at the call's site, as scalac pickles the call, whether the
// expansion is a literal the macro built or a quote with its span in this file.
object Cls:
  def join(parts: Seq[String]): String = parts.mkString(" ")
  inline def classes(inline parts: String*): String = ${ classesImpl('parts) }
  def classesImpl(parts: Expr[Seq[String]])(using Quotes): Expr[String] =
    '{ Cls.join($parts) }
  extension (inline sc: StringContext) inline def tw(inline args: Any*): String = ${ twImpl('sc) }
  def twImpl(sc: Expr[StringContext])(using Quotes): Expr[String] = sc match
    case '{ StringContext(${Varargs(parts)}*) } => Expr(parts.map(_.valueOrAbort).mkString)
    case _ => Expr("")
