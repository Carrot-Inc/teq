// One binder for a quoted MethodType (Types.MethodType): the parameters callback's binder, kept, reads
// the parameter types set after it, as the result callback sees them.
import scala.quoted.*
object Macros:
  inline def check: Boolean = ${impl}
  def impl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    var saved: Option[MethodType] = None
    val mt = MethodType(List("x"))(
      m => { saved = Some(m); List(TypeRepr.of[String]) },
      m => if saved.get.paramTypes.head =:= TypeRepr.of[String] then TypeRepr.of[String] else TypeRepr.of[Int])
    Expr(mt.resType =:= TypeRepr.of[String])
