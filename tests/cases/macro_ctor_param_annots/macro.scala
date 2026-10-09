// The annotations of a class parameter belong to the constructor's parameter, not to the
// field of its name, as the reflection API shows them.
import scala.quoted.*

final case class note(text: String) extends scala.annotation.StaticAnnotation
case class Address(street: String, @note("field") city: String, zip: Int = 0)
class Plain(@note("plain") val a: Int, b: String)

inline def probe[T]: String = ${ probeImpl[T] }
def probeImpl[T: Type](using Quotes): Expr[String] =
  import quotes.reflect.*
  val sym = TypeRepr.of[T].typeSymbol
  val params = sym.primaryConstructor.paramSymss.flatten
    .map(p => s"${p.name}:${p.annotations.size}:${p.isValDef}").mkString(",")
  val fields = sym.declaredFields.map(f => s"${f.name}:${f.annotations.size}").mkString(",")
  Expr(params + " | " + fields)
