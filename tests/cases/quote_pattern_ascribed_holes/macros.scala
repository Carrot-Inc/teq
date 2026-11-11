import scala.quoted.*

// A quote pattern's leaves are ascribed the type they are expected at, and an ascription is matched by
// its expression on either side, the scrutinee's where the hole's type does not fit it (QuoteMatcher's
// type ascription arms); a type application matches one whose arguments conform, a type reference
// one of its symbol through qualifiers that match, a sequence hole an argument spread into the repeated
// parameter, a hole only a term that refers to none of the pattern's own binders; `Expr.matches` takes its
// receiver as the scrutinee.
trait Box { type T }

object M:
  inline def d(inline x: Any): String = ${ dImpl('x) }

  def dImpl(x: Expr[Any])(using Quotes): Expr[String] =
    val typed = x match
      case '{ $p: Int } => "int"
      case _ => "other"
    val any = x match
      case '{ $p } => "any"
      case _ => "none"
    val sum = x match
      case '{ ($p: Int) + 1 } => "plus one"
      case _ => "no sum"
    val seq = x match
      case '{ List[Int]($xs*) } =>
        xs match
          case Varargs(es) => s"list of ${es.size}"
          case _ => "list"
      case '{ List[Any]($xs*) } => "list of any"
      case '{ ($s: String).length } => "length"
      case '{ val b: Box = $bx; List.empty[b.T] } => "path through a local"
      case _ => "no list"
    val closed = x match
      case '{ (y: Int) => ($body: Any) } => s"closed ${body.show}"
      case _ => "open"
    Expr(List(typed, any, sum, seq, closed).mkString(", "))

  inline def directions(): String = ${ directionsImpl }

  def directionsImpl(using Quotes): Expr[String] =
    val wider = '{ List.empty[Any] }.matches('{ List.empty[Int] })
    val narrower = '{ List.empty[Int] }.matches('{ List.empty[Any] })
    Expr(s"$wider $narrower")
