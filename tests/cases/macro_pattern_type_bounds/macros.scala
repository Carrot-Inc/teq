import scala.quoted.*

// A quote pattern's type variables are solved as dotty's `QuoteMatcher` solves them: each one a
// GADT-constrained variable within the bounds it declares (`instrumentTypeHoles`), every subtype
// check of the match adding to them by the variances of the types it decomposes, a bound naming
// another variable relating the two, and each one's type chosen from its full bounds once the
// whole tree has matched (`typeHoleApproximation`), in any order of their occurrences.
object Kinds:
  inline def kind(inline e: Any): String = ${ kindImpl('e) }
  def kindImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ type t <: AnyVal; Some($v: t) } => Expr("value")
    case '{ type t <: CharSequence; Some($v: t) } => Expr("chars")
    case '{ type a; type b <: a; ($x: a, $y: b) } => Expr("narrowing pair")
    case _ => Expr("other")

  // `Some` is covariant: `Some[Int]` is a `Some[t]` for a `t` above both `Int` and `String`.
  inline def lower(inline x: Any): String = ${ lowerImpl('x) }
  def lowerImpl(x: Expr[Any])(using Quotes): Expr[String] = x match
    case '{ type t >: String; Some($v: t) } => Expr("matched")
    case _ => Expr("other")

  // `Array` is invariant: `b` is the first array's element type, `a` the second's, and `b <: a`
  // has to hold between them, whichever occurrence comes first.
  inline def dependent(inline x: Any): String = ${ dependentImpl('x) }
  def dependentImpl(x: Expr[Any])(using Quotes): Expr[String] = x match
    case '{ type a; type b <: a; ($x: Array[b], $y: Array[a]) } => Expr("matched")
    case _ => Expr("other")

  inline def reversed(inline x: Any): String = ${ reversedImpl('x) }
  def reversedImpl(x: Expr[Any])(using Quotes): Expr[String] = x match
    case '{ type a; type b <: a; ($y: Array[a], $x: Array[b]) } => Expr("matched")
    case _ => Expr("other")

  // A covariant list of a bounded variable: every element type under the bound.
  inline def values(inline x: Any): String = ${ valuesImpl('x) }
  def valuesImpl(x: Expr[Any])(using Quotes): Expr[String] = x match
    case '{ type t <: AnyVal; $xs: List[t] } => Expr("values")
    case _ => Expr("other")

  // A variable the pattern binds in a contravariant position takes its upper bound, which
  // dotty's typer marks it for (`@fromAbove`): a function's parameter type.
  inline def param(inline x: Any): String = ${ paramImpl('x) }
  def paramImpl(x: Expr[Any])(using Quotes): Expr[String] = x match
    case '{ $f: (t => Int) } => Expr(Type.show[t])
    case _ => Expr("other")
