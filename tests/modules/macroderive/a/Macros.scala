package idlib

import scala.deriving.Mirror
import scala.quoted.*

trait LongKey[E]:
  def keyName: String
  def wrap(value: Long): E
  extension (e: E)
    def value: Long

object LongKey:
  inline def apply[E](using inst: LongKey[E]): inst.type = inst
  transparent inline def derived[E]: LongKey[E] = ${ Macros.derivedLongKey[E] }

trait Labelled[E]:
  def enumName: String
  extension (e: E)
    def entryName: String

object Labelled:
  inline def apply[E](using mirror: Labelled[E]): mirror.type = mirror
  transparent inline def derived[E]: Labelled[E] = ${ Macros.derivedLabelled[E] }

trait Enumerated[E]:
  def enumName: String
  def size: Int
  def values: IArray[E]
  def valueList: List[E] = values.toList
  def valueOf(name: String): Option[E] = values.find(_.entryName == name)
  def valueOfIgnoreCase(name: String): Option[E] = values.find(_.entryName.equalsIgnoreCase(name))
  def fromOrdinal(ordinal: Int): Option[E] = values.lift(ordinal)
  extension (e: E)
    def ordinal: Int
    def entryName: String

object Enumerated:
  inline def apply[E](using mirror: Enumerated[E]): mirror.type = mirror
  transparent inline def derived[E]: Enumerated[E] = ${ Macros.derivedEnumerated[E] }

object Macros:
  def string[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val ConstantType(StringConstant(str)) = TypeRepr.of[T]: @unchecked
    Expr(str)

  def names[T: Type](using Quotes): List[Expr[String]] = Type.of[T] match
    case '[EmptyTuple] => Nil
    case '[t *: ts]    => string[t] :: names[ts]

  def derivedLongKey[E: Type](using Quotes): Expr[LongKey[E]] =
    import quotes.reflect.*
    val tpe = TypeRepr.of[E]
    val sym = tpe.classSymbol match
      case Some(sym) if sym.caseFields.length == 1 => sym
      case _ => report.errorAndAbort(s"${tpe.show} is not a single-field case class type")
    val field = sym.caseFields.head
    def reifyApply(name: Expr[Long]) =
      Select.overloaded(Ref(sym.companionModule), "apply", List.empty, name.asTerm :: List.empty).asExprOf[E]
    def reifyValue(inst: Expr[E]) =
      Select(inst.asTerm, field).asExprOf[Long]
    val keyNameExpr = Expr(sym.fullName)
    '{
      new LongKey[E]:
        final def keyName: String = $keyNameExpr
        final def wrap(value: Long): E = ${ reifyApply('value) }
        extension (e: E)
          final def value: Long = ${ reifyValue('e) }
    }

  private def enumSymbol[E: Type](using Quotes): quotes.reflect.Symbol =
    import quotes.reflect.*
    val tpe = TypeRepr.of[E]
    tpe.classSymbol match
      case Some(sym) if sym.flags.is(Flags.Enum) && !sym.flags.is(Flags.JavaDefined) => sym
      case _ => report.errorAndAbort(s"${tpe.show} is not an enum type")

  def derivedLabelled[E: Type](using Quotes): Expr[Labelled[E]] =
    import quotes.reflect.*
    val sym = enumSymbol[E]
    val enumNameExpr = Expr(sym.fullName)
    def reifyLabel(e: Expr[E & scala.reflect.Enum]) =
      val labelSymb = sym.declaredField("entryName")
      val hasLabel = labelSymb.exists && (sym.typeRef.memberType(labelSymb) <:< TypeRepr.of[String])
      if hasLabel then Select(e.asTerm, labelSymb).asExprOf[String]
      else Select.unique(e.asTerm, "productPrefix").asExprOf[String]
    '{
      new Labelled[E]:
        final def enumName: String = $enumNameExpr
        extension (e: E & scala.reflect.Enum)
          final def entryName: String = ${ reifyLabel('e) }
    }

  def derivedEnumerated[E: Type](using Quotes): Expr[Enumerated[E]] =
    import quotes.reflect.*
    val sym = enumSymbol[E]
    val tpe = TypeRepr.of[E]
    val M = Expr.summon[Mirror.SumOf[E]] match
      case Some(mirror) => mirror
      case None => report.errorAndAbort(s"Could not summon a Mirror.SumOf[${tpe.show}]")
    val E = sym.companionModule
    val valuesRef = Select.unique(Ref(E), "values").asExprOf[Array[E & reflect.Enum]]
    val sizeExpr = Expr(sym.children.length)
    val enumNameExpr = Expr(sym.fullName)
    def reifyLabel(e: Expr[E & scala.reflect.Enum]) =
      val labelSymb = sym.declaredField("entryName")
      val hasLabel = labelSymb.exists && (sym.typeRef.memberType(labelSymb) <:< TypeRepr.of[String])
      if hasLabel then Select(e.asTerm, labelSymb).asExprOf[String]
      else Select.unique(e.asTerm, "productPrefix").asExprOf[String]
    '{
      new Enumerated[E]:
        private val _values: IArray[E & reflect.Enum] = IArray.unsafeFromArray($valuesRef)
        private val _names = _values.map[String]((e: E & reflect.Enum) => ${ reifyLabel('e) })
        locally:
          assert(_values.length == $sizeExpr)
          assert(_names.size == $sizeExpr)
          assert((_values: IndexedSeq[E & reflect.Enum]).map(_.ordinal).corresponds(_values.indices)(_ == _))
        final def enumName: String = $enumNameExpr
        final def size: Int = $sizeExpr
        final def values: IArray[E] = _values
        extension (e: E & scala.reflect.Enum)
          final def ordinal: Int = e.ordinal
          final def entryName: String = ${ reifyLabel('e) }
    }
