// `TypeBoundsTree` made and copied with bounds of its own: the trees keep the bounds they are given.
import scala.quoted.*

inline def bounds: String = ${ boundsImpl }

def boundsImpl(using Quotes): Expr[String] =
  import quotes.reflect.*
  val orig = TypeBoundsTree(Inferred(TypeRepr.of[Nothing]), Inferred(TypeRepr.of[Any]))
  val copied = TypeBoundsTree.copy(orig)(Inferred(TypeRepr.of[String]), Inferred(TypeRepr.of[AnyRef]))
  val made = TypeBoundsTree(Inferred(TypeRepr.of[Int]), Inferred(TypeRepr.of[AnyVal]))
  val checks = List(
    copied.low.tpe =:= TypeRepr.of[String],
    copied.hi.tpe =:= TypeRepr.of[AnyRef],
    made.low.tpe =:= TypeRepr.of[Int],
    made.hi.tpe =:= TypeRepr.of[AnyVal],
    orig.low.tpe =:= TypeRepr.of[Nothing],
    orig.hi.tpe =:= TypeRepr.of[Any],
  )
  Expr(checks.mkString(" "))
