// The kinds of constant are abstract types under `Constant` (`Quotes.reflectModule`), a type pattern over one going
// through its `TypeTest`, which tells the kind by the value (`QuotesImpl.IntConstantTypeTest`): a string constant is
// no `IntConstant`.
import scala.quoted.*
object M:
  inline def test: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val cs: List[Constant] = List(StringConstant("s"), IntConstant(1), BooleanConstant(true), NullConstant(), UnitConstant())
    val kinds = cs.map {
      case _: IntConstant => "int"
      case _: StringConstant => "string"
      case BooleanConstant(b) => "bool " + b
      case _: NullConstant => "null"
      case c => "other " + c.show
    }
    val i: IntConstant = IntConstant(2)
    Expr(kinds.mkString(",") + ";" + i.value)
