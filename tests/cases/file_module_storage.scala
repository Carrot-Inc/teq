//> using platform jvm
// split-watch: jvm
// A file module's storage as dotty's Memoize and Erasure leave it: an inline val has no field and no getter
// (`isEffectivelyErased`), a final val of a constant type as its result type is written, before any dealiasing
// (`isConstExprFinalVal`), has a getter answering the constant and no field, so a read during the module's
// initialisation, by reflection, gives the constant; a val written of an alias of a literal type keeps its field and
// gives the default before its initialiser runs, as a plain val does. Read by reflection, on the JVM.
package storage

type Answer = 42

import scala.reflect.Selectable.reflectiveSelectable

// The module through its field, its getters by name (scala-library's `selectDynamic`).
def module: Any = Class.forName("storage.file_module_storage$package$").getDeclaredField("MODULE$").get(null)

val before = module.asInstanceOf[{ def tanswer: Int }].tanswer
val before2 = module.asInstanceOf[{ def tlit: Int }].tlit
val before3 = module.asInstanceOf[{ def tlit2: Int }].tlit2
val before4 = module.asInstanceOf[{ def plain: Int }].plain
inline val inlined = 41
final val tanswer: Answer = { println("alias initialiser"); 42 }
final val tlit = 43
final val tlit2: 44 = { println("literal type initialiser"); 44 }
val plain = 45

@main def run(): Unit =
  println(s"$before $before2 $before3 $before4")
  println(s"$inlined $tanswer $tlit $tlit2 $plain")
  val c = Class.forName("storage.file_module_storage$package$")
  println(c.getDeclaredFields.map(_.getName).sorted.mkString(","))
  val written = Set("writeReplace", "module")
  println(c.getDeclaredMethods.map(_.getName).filter(n => !n.contains("$") && !written(n)).sorted.mkString(","))
