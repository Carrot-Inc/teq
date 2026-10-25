// An opaque alias is a type symbol, never a class (SymDenotations.scala 705): scalac's reflection gives
// `class=false; typedef=true`, teq's `class=true; typedef=false`. Moves with type-representation's landing 4.
import scala.quoted.*
object OpaqueReflect:
  inline def flags[T]: String = ${ impl[T] }
  def impl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val s = TypeRepr.of[T].typeSymbol
    Expr("class=" + s.isClassDef + "; typedef=" + s.isTypeDef + "; opaque=" + s.flags.is(Flags.Opaque))
