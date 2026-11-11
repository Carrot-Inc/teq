// The jar of tests/cases/library_quote_pattern_outer_type.scala, library_quote_pattern_nested_fresh.scala,
// library_type_pattern_from_above.scala, library_quote_pattern_enclosing_given.scala and
// library_quote_pattern_bound_reference.scala: a macro whose nested quote pattern names the type an enclosing pattern
// bound (`` `o` ``), and splices the expressions both bound; one whose nested pattern binds a fresh variable of the
// enclosing one's name; a type pattern binding a type in a contravariant position; a nested pattern naming the type an
// enclosing typed pattern bound; a declared binding bounded by an enclosing pattern's type.
package qnest
import scala.quoted.*

final case class Pipe[-I, +E, +O](name: String)
object Pipe:
  def compose[I, E, O, O2](x: Pipe[I, E, O], y: Pipe[O, E, O2]): Pipe[I, E, O2] = Pipe(x.name + ">" + y.name)

object Mac:
  inline def joined[E](inline x: Pipe[?, E, ?], inline y: Pipe[?, E, ?]): String = ${ joinImpl[E]('x, 'y) }
  def joinImpl[E: Type](x: Expr[Pipe[?, E, ?]], y: Expr[Pipe[?, E, ?]])(using Quotes): Expr[String] =
    def composeV(lhs: Expr[Pipe[?, E, ?]], rhs: Expr[Pipe[?, E, ?]]): Expr[Pipe[?, E, ?]] = lhs match
      case '{ $l: Pipe[i, E, o] } =>
        rhs match
          case '{ $r: Pipe[`o`, E, o2] } => '{ Pipe.compose[i, E, o, o2]($l, $r) }
          case _ => quotes.reflect.report.errorAndAbort("no inner match")
    '{ ${ composeV(x, y) }.name }

// A nested type pattern binding a variable of the enclosing pattern's name, `case '[t] => .. case '[t] =>`: a fresh
// variable of the inner pattern (tests/cases/library_quote_pattern_nested_fresh.scala).
object Fresh:
  inline def result: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    Type.of[Int] match
      case '[t] =>
        Type.of[String] match
          case '[t] => Expr(1)
          case _ => Expr(0)

// A type pattern binding a type in a contravariant position, `'[Pipe[in, e, out]]`, which scalac marks `@fromAbove` and
// a 28.4 pickle carries no annotation of (tests/cases/library_type_pattern_from_above.scala).
object Types:
  inline def inOf[A]: String = ${ impl[A] }
  def impl[A: Type](using Quotes): Expr[String] =
    Type.of[A] match
      case '[Pipe[in, e, out]] => Expr(Type.show[in] + " | " + Type.show[out])
      case _ => Expr("no match")

// A nested pattern naming the type an enclosing typed pattern bound, `case given Type[t] => .. case '[`t`] =>`
// (tests/cases/library_quote_pattern_enclosing_given.scala).
object Backquoted:
  inline def against[A]: Boolean = ${ impl[A] }
  def impl[A: Type](using Quotes): Expr[Boolean] = Expr(check(Type.of[Int]))
  def check[A: Type](tp: Type[?])(using Quotes): Boolean = tp match
    case given Type[t] =>
      Type.of[A] match
        case '[`t`] => true
        case _ => false

// A pattern's declared binding bounded by the type an enclosing pattern bound, `type u <: `t``
// (tests/cases/library_quote_pattern_bound_reference.scala).
object Bounded:
  inline def test(inline x: Any): Boolean = ${ impl('x) }
  def impl(x: Expr[Any])(using Quotes): Expr[Boolean] =
    Type.of[Int] match
      case '[t] =>
        x match
          case '{ type u <: `t`; $v: u } =>
            val widened: Expr[t] = v
            Expr(true)
          case _ => Expr(false)
