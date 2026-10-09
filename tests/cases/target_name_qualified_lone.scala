// A method not overloaded named apart by `@scala.annotation.targetName` written out, with no import: on the JVM
// `Lib$.renamed(I)I`, as scalac names it, the annotation resolved before the reach, whose tables then cover the
// class the resolution entered.
package probe
object Lib:
  @scala.annotation.targetName("renamed") def f(x: Int): Int = x + 1

object Main:
  def main(args: Array[String]): Unit =
    println(Lib.f(41))
