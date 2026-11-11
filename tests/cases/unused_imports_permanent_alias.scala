// teq: --wunused imports --werror
// An import of `scala.AnyRef`, a permanent symbol of Definitions without a source (dotty's `NoSource`, Symbols.scala:
// 316-334): CheckUnused compares a symbol's source with the unit's (CheckUnused.scala:448-456), and a symbol with
// none is of no source, so the reference is the import's use and nothing is reported (scalac 3.8.4 with
// -Wunused:imports -Werror: no diagnostic).
import scala.AnyRef

class A:
  val x: AnyRef = "s"

@main def run(): Unit =
  println(new A().x)
