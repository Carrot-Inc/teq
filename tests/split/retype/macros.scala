package retype

import scala.quoted.*

/** What a macro's run makes anew every time, an anonymous class: this file is typed when the
  * first macro runs, in the middle of its caller's expansion. */
trait Rule:
  def keeps(word: String): Boolean

object Rules:
  def longerThan(n: Int): Rule = new Rule:
    def keeps(word: String): Boolean = word.length > n

object Macros:
  def countImpl(text: Expr[String])(using Quotes): Expr[Int] =
    Expr(new Tally(text.valueOrAbort).words.count(Rules.longerThan(0).keeps))

  def shapeImpl(text: Expr[String])(using Quotes): Expr[String] =
    val tally = new Tally(text.valueOrAbort)
    val groups = tally.words.groupBy(_.length).toList.sortBy(_._1).map((n, ws) => n.toString + ":" + ws.length)
    Expr(groups.mkString(separator) + separator + tally.longest)

  def wordsImpl(sc: Expr[StringContext])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val parts = sc match
      case '{ StringContext(${ Varargs(ps) }*) } => ps.map(_.valueOrAbort)
      case _ => report.errorAndAbort("words needs a literal", sc)
    Expr(new Tally(parts.mkString(" ")).size)

  /** The value of a side, which has to be the one its caller expects. */
  def pickImpl(side: Expr[String], expected: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val value = side.valueOrAbort match
      case "left" => Left.n + leftTop
      case _ => Right.n + rightTop
    if value != expected.valueOrAbort then report.errorAndAbort(s"${side.valueOrAbort} is $value", side)
    Expr(value)

inline def count(inline text: String): Int = ${ Macros.countImpl('text) }

inline def shape(inline text: String): String = ${ Macros.shapeImpl('text) }

extension (inline sc: StringContext)
  inline def words(inline args: Any*): Int = ${ Macros.wordsImpl('sc) }

inline def pick(inline side: String, inline expected: Int): Int = ${ Macros.pickImpl('side, 'expected) }

inline def doubled(n: Int): Int = n + n
