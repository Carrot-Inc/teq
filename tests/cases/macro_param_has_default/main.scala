// The HasDefault flag of a case class's constructor parameters as paramSymss hands them out.
package app
import mlib.Defaults

final case class WithDefaults(id: Int, name: String, score: Double = 1.5, tags: List[String] = Nil)

@main def run(): Unit =
  println(Defaults.withDefault[WithDefaults])
