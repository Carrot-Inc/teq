// The default getters of a case class through the reflection API, as Magnolia reads them: the
// companion's `$lessinit$greater$default$N` selected on the companion module, on a case class
// that declares no companion too; and `dealias` keeping a term reference.
import scala.quoted.*

case class Address(street: String, city: String = "Springfield", zip: Option[String] = None)
case class Bare(x: Int, y: Int = 3)
object Address:
  def make: Address = Address("1 Main")
enum Color:
  case Red, Green

inline def defaults[T]: List[(String, Option[Any])] = ${ defaultsImpl[T] }
def defaultsImpl[T: Type](using Quotes): Expr[List[(String, Option[Any])]] =
  import quotes.reflect.*
  val tpe = TypeRepr.of[T].typeSymbol
  val terms = tpe.primaryConstructor.paramSymss.flatten.filter(_.isValDef).zipWithIndex.map { (field, i) =>
    val name = Expr(field.name)
    tpe.companionClass.declaredMethod(s"$$lessinit$$greater$$default$$${i + 1}").headOption match
      case Some(m) =>
        val call = Ident(tpe.companionModule.termRef).select(m).asExprOf[Any]
        '{ ($name, Some($call)) }
      case None => '{ ($name, None) }
  }
  Expr.ofList(terms)

inline def termName[T]: String = ${ termNameImpl[T] }
def termNameImpl[T: Type](using Quotes): Expr[String] =
  import quotes.reflect.*
  TypeRepr.of[T].dealias match
    case TermRef(_, name) => Expr("term " + name)
    case other => Expr("type " + other.typeSymbol.name)
