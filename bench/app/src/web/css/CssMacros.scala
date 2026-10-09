package meridian.web.css

import scala.quoted.*

/** The three entry points over the validator of `Validator.scala` (the class catalog, its
  * tokens and its checks): every literal in the arguments is checked at compile time, and the
  * expansion is the joining call over the arguments as they were written. */
object CssMacros:
  def clsImpl(classes: Expr[Seq[ClassArg]])(using Quotes): Expr[meridian.web.vdom.Mod] =
    validate(classes)
    '{ addClasses($classes) }

  def classListImpl(classes: Expr[Seq[ClassArg]])(using Quotes): Expr[Tw] =
    validate(classes)
    '{ concatClasses($classes) }

  def twImpl(sc: Expr[StringContext], args: Expr[Seq[Any]])(using Quotes): Expr[Tw] =
    import quotes.reflect.*
    val parts = sc match
      case '{ StringContext(${ Varargs(ps) }*) } => ps.map(_.valueOrAbort)
      case _ => report.errorAndAbort("tw needs a literal string", sc)
    if parts.length > 1 then report.errorAndAbort("tw takes no interpolated arguments", sc)
    check(parts.toList)
    val text = Expr(parts.mkString(" ").split("\\s+").filter(_.nonEmpty).mkString(" "))
    '{ Tw.unsafe($text) }

  private def validate(classes: Expr[Seq[ClassArg]])(using Quotes): Unit =
    import quotes.reflect.*
    val literals = classes match
      case Varargs(items) => items.toList.flatMap(literalsOf)
      case _ => report.errorAndAbort("class arguments have to be written out", classes)
    check(literals)

  /** The literal strings inside an argument: a string, a `text -> flag` pair or a `tw` value,
    * whose text a `tw` expansion already checked and a non-literal cannot be checked. */
  private def literalsOf(arg: Expr[Any])(using Quotes): List[String] =
    import quotes.reflect.*
    arg.asTerm match
      case Literal(StringConstant(s)) => List(s)
      case Inlined(_, _, inner) => literalsOf(inner.asExpr)
      case Typed(inner, _) => literalsOf(inner.asExpr)
      case Apply(TypeApply(Select(Apply(TypeApply(Ident("ArrowAssoc"), _), List(left)), "->"), _), List(_)) => literalsOf(left.asExpr)
      case Apply(TypeApply(Select(Ident("Tuple2"), "apply"), _), List(left, _)) => literalsOf(left.asExpr)
      case Apply(TypeApply(Select(Ident("ArrowAssoc"), "apply"), _), List(left)) => literalsOf(left.asExpr)
      case Apply(Select(Apply(TypeApply(Ident("ArrowAssoc"), _), List(left)), "->"), List(_)) => literalsOf(left.asExpr)
      case _ => Nil

  private def check(literals: List[String])(using Quotes): Unit =
    import quotes.reflect.*
    if literals.nonEmpty then
      val source = Position.ofMacroExpansion.sourceFile.getJPath.getOrElse(report.errorAndAbort("no source path"))
      val catalog = Catalog.near(source).getOrElse(report.errorAndAbort("no classes.txt next to the sources"))
      val tokens = literals.flatMap(Token.parseAll)
      val errors = tokens.flatMap(Checks.token(_, catalog)) ++ Checks.cross(tokens, catalog)
      if errors.nonEmpty then report.errorAndAbort(errors.distinct.mkString("\n"), Position.ofMacroExpansion)
