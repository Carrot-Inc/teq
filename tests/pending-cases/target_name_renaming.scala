// `@targetName` renames every symbol it annotates in the class files
// (`SymDenotation.targetName`, `computeTargetName`, SymDenotations.scala
// 558-568): a val's getter and field, a var's (its setter keeps `w_$eq`), a class (the type arm,
// `str.toTypeName`) and an object (`.moduleClassName`). teq is to name them as the .expected has them.
// Queued as target-name-renaming, beside the erasure work.
//> using platform jvm
package target_name_renaming

import scala.annotation.targetName

class C:
  @targetName("vv") val v: Int = 1
  @targetName("ww") var w: Int = 2

object H:
  @targetName("KK") class K { def k = 5 }
  @targetName("OO") object O { def o = 6 }

@main def main(): Unit =
  val methods = classOf[C].getDeclaredMethods.map(_.getName).filter(n => !n.startsWith("$")).sorted.mkString(",")
  val fields = classOf[C].getDeclaredFields.map(_.getName).sorted.mkString(",")
  println(methods)
  println(fields)
  println(classOf[H.K].getName)
  println(H.O.getClass.getName)
