import scala.quoted.*

type Boxed = [A] =>> List[A]
class Carrier[F[_]]

object RebuildLambda:
  // Complete the written alias while preparing the macro implementation.
  def prefixUse(value: Boxed[Int]): List[Int] = value

  inline def check: String = ${ checkImpl }

  def checkImpl(using q: Quotes): Expr[String] =
    import q.reflect.*
    // Keep the lambda from the typed argument. Dealiasing Boxed
    // here could construct it again.
    val original = TypeRepr.of[Carrier[[A] =>> List[A]]].typeArgs.head.asInstanceOf[TypeLambda]
    val rebuilt = TypeLambda(
      original.paramNames,
      _ => original.paramBounds,
      self => TypeRepr.of[List[Int]].asInstanceOf[AppliedType].tycon.appliedTo(self.param(0)))
    val bounds = rebuilt.paramBounds.head
    val bounded = TypeLambda(
      List("S"),
      _ => List(TypeBounds(TypeRepr.of[Nothing], TypeRepr.of[String])),
      self => self.param(0))
    val bound = bounded.paramBounds.head
    Expr(
      (bounds.low =:= TypeRepr.of[Nothing]).toString + " " +
      (bounds.hi =:= TypeRepr.of[Any]).toString + " " +
      (original =:= rebuilt).toString + " " + (original == rebuilt).toString + "\n" +
      (bound.low =:= TypeRepr.of[Nothing]).toString + " " +
      (bound.hi =:= TypeRepr.of[String]).toString)
