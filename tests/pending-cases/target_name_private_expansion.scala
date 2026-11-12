// A private member reached from outside its class is made non-private at erasure
// (`ensureNotPrivate`) under its expanded name, the expansion over its target name
// (`expandedName` after Erasure's renaming): scalac names `@targetName("pp") private def p`
// reached from the companion `target_name_private_expansion$C$$pp`, and teq
// is to name it so. Queued as target-name-renaming, beside the erasure work.
//> using platform jvm
package target_name_private_expansion

import scala.annotation.targetName

class C:
  @targetName("pp") private def p: Int = 3

object C:
  def viaCompanion(c: C): Int = c.p

@main def main(): Unit =
  println(C.viaCompanion(new C))
  println(classOf[C].getDeclaredMethods.map(_.getName).filter(_.contains("pp")).sorted.mkString(","))
