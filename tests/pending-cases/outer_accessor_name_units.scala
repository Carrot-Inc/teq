// An inner class's outer accessor is `nme.OUTER.expandedName(cls)` (ExplicitOuter.scala 195-196), over the class's own
// name, its parts' code units put together: scalac names it `outer_accessor_name_units$O$A$I$$$outer` whatever `A`'s
// target name, which names the class file alone (`O$A$uD83D`). teq's JVM branch builds the accessor's name from
// `scala_name`'s text (src/typer/anon.rs, `scalac_outer_accessor_name`), which would show U+FFFD for a lone unit of
// the class's own name: a name a class file gives once its names keep their units (classfile-reader-names; table
// (c)'s c-77 on the M2e refold). This program's first line also waits on target-name-renaming. Queued as
// classfile-reader-names (the coordinator's ruling of 28 Oct).
//> using platform jvm
package outer_accessor_name_units

import scala.annotation.targetName

object O:
  @targetName("A\uD83D") class A:
    class I:
      def up: A = A.this

@main def main(): Unit =
  val esc = (s: String) => s.flatMap(c => if Character.isSurrogate(c) then f"\\u${c.toInt}%04X" else c.toString)
  println(esc(classOf[O.A].getName))
  println(classOf[O.A#I].getDeclaredMethods.map(m => esc(m.getName)).sorted.mkString(","))
