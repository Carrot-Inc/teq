// jars: scala-library izumi-reflect izumi-reflect-boopickle
// Two classes of a jar that scalac accepted and teq's check of double definitions did not:
// `Inspector`'s `extractVariance` overloads, told apart by a `@targetName`, and
// `FullReference`'s deprecated `copy$default$1: String` beside the default getter of its
// explicit `copy`. A jar's class is not checked again; what erases alike is named apart.
import izumi.reflect.dottyreflection.Inspector
import izumi.reflect.macrortti.LightTypeTagRef.{FullReference, SymName}
object Main:
  def shift(i: Inspector): Int = i.context.length
  def main(args: Array[String]): Unit =
    val r = FullReference(SymName.SymTypeName("a.B"), Nil, None)
    println(r.copy(parameters = Nil).symName)
