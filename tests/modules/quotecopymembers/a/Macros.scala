package ema

import scala.quoted.*

object Macros:
  def derivedEnumMirror[E: Type](using Quotes): Expr[EnumMirror[E]] =
    import quotes.reflect.*
    val sym = TypeRepr.of[E].typeSymbol
    val valuesRef = Select.unique(Ref(sym.companionModule), "values").asExprOf[Array[E & scala.reflect.Enum]]
    val sizeExpr = Expr(sym.children.size)
    val mirroredNameExpr = Expr(sym.fullName)
    '{
      new EnumMirror[E]:
        private val _values: IArray[E & scala.reflect.Enum] = IArray.unsafeFromArray($valuesRef)
        final def mirroredName: String = $mirroredNameExpr
        final def size: Int = $sizeExpr
        final def values: IArray[E] = _values
        extension (e: E & scala.reflect.Enum)
          final def ordinal: Int = e.ordinal
          final def entryName: String = e.productPrefix
    }
