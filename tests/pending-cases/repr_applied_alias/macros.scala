// An applied parameterized alias is kept as the alias's reference with its
// arguments (`AppliedType(TypeRef(.., G), args)`), unless `appliedTo` reduces it
// (TypeApplications.scala 381-399: a body that applies a class to the alias's own parameters
// in order, an eta-reduction, or a compile-time operation): scalac's reflection sees
// `ImpureFunction1[Int, Int]` and `A.G[Int]`, and `F[Int]` reduced to `List[Int]`. teq's
// reflection is to see them too, and its pickles are to write `ImpureFunction1[..]` where
// scalac writes it. Moves with type-representation (the alias reference kept until dealias).
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure))
