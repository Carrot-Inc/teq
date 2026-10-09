package msa

import scala.annotation.tailrec
import scala.quoted.*

object Mac:
  inline def joined(inline s: String): String = ${ joinedImpl('s) }

  def joinedImpl(s: Expr[String])(using Quotes): Expr[String] =
    List(s.valueOrAbort, "x") match
      case List(a, b) => Expr(a + b)
      case _ => Expr("")

  // The arguments of a call classified by reflection patterns of sequences, through a local
  // extractor object with a guard (the shape of a class-string validating macro).
  inline def kinds(inline xs: Any*): String = ${ kindsImpl('xs) }

  def kindsImpl(xs: Expr[Seq[Any]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    case class Found(value: String, conditional: Boolean)
    @tailrec
    def shallow(t: Term): Term = t match
      case Typed(inner, _) => shallow(inner)
      case Block(Nil, inner) => shallow(inner)
      case TypeApply(Select(inner, "$asInstanceOf$"), _) => shallow(inner)
      case _ => t
    @tailrec
    def strip(t: Term): Term = shallow(t) match
      case Inlined(_, _, inner) => strip(inner)
      case other => other
    object Pair:
      def unapply(t: Term): Option[(Term, Term)] = t match
        case Apply(TypeApply(Select(tuple, "apply"), _), List(a, b)) if tuple.symbol.fullName == "scala.Tuple2" => Some((a, b))
        case Apply(TypeApply(Select(arrow, "->"), _), List(b)) =>
          strip(arrow) match
            case Apply(_, List(a)) => Some((a, b))
            case _ => None
        case _ => None
    def classify(t: Term, conditional: Boolean): Option[Found] =
      strip(t) match
        case Literal(StringConstant(v)) => Some(Found(v, conditional))
        case Pair(a, _) if !conditional => classify(a, conditional = true)
        case _ => None
    val parts = strip(xs.asTerm) match
      case Repeated(elems, _) => elems
      case _ => List.empty
    Expr(parts.map(p => classify(p, conditional = false).fold("none")(f => f.value + (if f.conditional then "?" else ""))).mkString(", "))
