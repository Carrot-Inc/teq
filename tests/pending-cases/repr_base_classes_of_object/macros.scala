// A class's base classes list the class `Object` its parent `Object` brings, `java.lang.Object` by its full name, and
// `Matchable` after it (`ClassDenotation.baseClasses` over the parents `ObjectType :: Nil` that
// `Definitions.newFunctionNType` gives a synthetic function class and `Desugar` a written one), a class's `typeRef` of
// type parameters the class's own (`Symbol.typeRef`), and a trait derives from `Object` (`derivesFrom`). teq's
// reflection gives `scala.AnyRef` and no `Matchable`, `scala.Any` alone for the `typeRef` of a class of type
// parameters, and no `Object` among a trait's bases to `derivesFrom` (the pass's probe names/M.scala on 39ba4cf2d, the
// first line, a synthetic function class's as a written trait's). Pinned for the coordinator's ruling.
import scala.quoted.*
object M {
 inline def inspect: String = ${ impl }
 def impl(using Quotes): Expr[String] = {
  import quotes.reflect.*
  val s = TypeRepr.of[scala.ContextFunction01[Int, Int]].typeSymbol
  Expr(s.typeRef.baseClasses.map(_.fullName).mkString(",") + " | " + s.declaredTypes.map(_.name).mkString(",") + " | " + s.declaredMethods.filter(_.name == "apply").map(x => x.paramSymss.flatten.map(_.name).mkString(",")).mkString(";"))
 }
 inline def bases[F]: String = ${ basesImpl[F] }
 def basesImpl[F: Type](using Quotes): Expr[String] = {
  import quotes.reflect.*
  val t = TypeRepr.of[F]
  val s = t.typeSymbol
  Expr(s.fullName + ": " + t.baseClasses.map(_.fullName).mkString(",") + " | " + s.typeRef.baseClasses.map(_.fullName).mkString(",") + " | " + t.derivesFrom(defn.ObjectClass))
 }
}
