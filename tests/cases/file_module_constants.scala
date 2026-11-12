//> using platform jvm
// split-watch: jvm
// Top-level final vals as dotty's `isConstExprFinalVal` sorts them by the result type dotty's typer records: a block
// with an effect, an ascribed literal, a folded sum, a class literal, a Float and a Double, a reference to another
// constant are constants (no field, the getter answering the constant, the block still run for its effect); an `if`
// is an `Int`, a field. Inline vals of array classes, erased, each read the `ldc` of the class. A `null` and a `()`
// are left out of the listing: scalac keeps no field for them either, by `Memoize`'s erasable bottom fields
// (tests/pending-cases/memoize_bottom_fields). Read by reflection, on the JVM.
package constants

final val block = { println("block"); 42 }
final val typed = (42: 42)
final val folded = 40 + 2
final val cls = classOf[String]
final val fl = 1.5f
final val dbl = 1.5d
final val nul = null
final val unt = ()
final val alias = folded
final val ifelse = if true then 42 else 43
inline val ints = classOf[Array[Int]]
inline val strs = classOf[Array[String]]
inline val nested = classOf[Array[Array[Long]]]

@main def run(): Unit =
  println(s"$block $typed $folded $cls $fl $dbl $nul $unt $alias $ifelse")
  println(s"${ints.getSimpleName} ${strs.getSimpleName} ${nested.getSimpleName} ${ints == Array(1).getClass}")
  val c = Class.forName("constants.file_module_constants$package$")
  println(c.getDeclaredFields.map(_.getName).filter(n => n != "nul" && n != "unt").sorted.mkString(","))
