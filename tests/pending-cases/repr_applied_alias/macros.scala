// An applied parameterized alias is kept as the alias's reference with its arguments (`AppliedType(TypeRef(.., G),
// args)`), unless `appliedTo` reduces it (TypeApplications.scala 381-399: a body that applies a class to the alias's own
// parameters in order, an eta-reduction, or a compile-time operation): scalac's reflection sees
// `ImpureFunction1[Int, Int]` and `A.G[Int]`, and `F[Int]` reduced to `List[Int]`. teq expands every applied alias
// outside semantic typing, and its pickles write `Function1[..]` where scalac writes `ImpureFunction1[..]` (the
// adversary's B-3 on the M2e refold). Moves with type-representation's landing 3 (seam 1, the alias
// reference kept until dealias).
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure))
