// A constant's kind is its tag (`QuotesImpl.CharConstantTypeTest`: `x.tag == CharTag`), never its
// value's class: a `Char` and a one-character `String` are told apart on every target, by the
// type tests and by the extractors.
import scala.quoted.*
object M:
  inline def test: String = ${impl}
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    def tag(c: Constant): String = c match
      case _: BooleanConstant => "Boolean"
      case _: ByteConstant => "Byte"
      case _: ShortConstant => "Short"
      case _: IntConstant => "Int"
      case _: LongConstant => "Long"
      case _: FloatConstant => "Float"
      case _: DoubleConstant => "Double"
      case _: CharConstant => "Char"
      case _: StringConstant => "String"
      case _: UnitConstant => "Unit"
      case _: NullConstant => "Null"
      case _: ClassOfConstant => "ClassOf"
      case _ => "other"
    def extracted(c: Constant): String = c match
      case CharConstant(ch) => s"char $ch"
      case StringConstant(s) => s"string $s"
      case IntConstant(i) => s"int $i"
      case _ => "other"
    val cs: List[Constant] = List(BooleanConstant(true), ByteConstant(1.toByte), ShortConstant(1.toShort), IntConstant(1),
      LongConstant(1L), FloatConstant(1.0f), DoubleConstant(1.0), CharConstant('a'), StringConstant("a"), UnitConstant(),
      NullConstant(), ClassOfConstant(TypeRepr.of[String]))
    val fromTrees = List(Literal(CharConstant('b')), Literal(StringConstant("b"))).map(t => extracted(t.constant))
    Expr(cs.map(tag).mkString(",") + "\n" + List(CharConstant('a'), StringConstant("a"), IntConstant(3)).map(extracted).mkString(",") + "\n" + fromTrees.mkString(","))
