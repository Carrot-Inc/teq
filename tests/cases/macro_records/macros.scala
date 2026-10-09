package records

import scala.quoted.*

class Pair(a: Int, b: Int):
  def sum: Int = a + b

object Sums:
  def add(a: Int)(b: Int): Int = a + b

/** A macro that keeps trees from its first expansion for its second, in the same build: a named
  * argument, a selection awaiting its arguments, a call applied to its first parameter clause,
  * and the method type of a constructor. */
object Keeper:
  var named: Any = null
  var selection: Any = null
  var partial: Any = null
  var ctorType: Any = null

  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    if named == null then
      named = NamedArg("x", Literal(IntConstant(1)))
      selection = Select.unique(Literal(StringConstant("abc")), "length")
      val add = Select.unique(Ref(Symbol.requiredModule("records.Sums")), "add")
      partial = Apply(add, List(Literal(IntConstant(20))))
      ctorType = TypeRepr.of[Pair].typeSymbol.primaryConstructor
    val name = named.asInstanceOf[NamedArg].name
    val length = Apply(selection.asInstanceOf[Term], Nil).asExprOf[Int]
    val sum = partial.asInstanceOf[Term] match
      case Apply(_, args) => args.length
      case _ => -1
    val params = TypeRepr.of[Pair].memberType(ctorType.asInstanceOf[Symbol]) match
      case MethodType(names, _, _) => names.mkString(",")
      case _ => "?"
    '{ ${ Expr(name) } + " " + ${ length } + " " + ${ Expr(sum) } + " " + ${ Expr(params) } }

inline def kept: String = ${ Keeper.impl }
