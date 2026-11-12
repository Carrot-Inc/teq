// jars: lazyval-lib
// A val of a jar's class that a subclass overrides with a val,
// in the jar's own body (scalacheck's `Test.Parameters.cpy`)
// and in the program, and a val read through an accessor
// method (one that implements a def, a `hashCode`).
import lvlib.*

class Gold extends Badge:
  override val label: String = "gold"
  override val hashCode: Int = 42

class Mine extends Params:
  val minSize: Int = 5
  override val spins: Int = 9

@main def run(): Unit =
  println(Params.default.describe)
  println(Params.default.withLegacy(false).describe)
  println(Mine().describe)
  println(Mine().withLegacy(false).legacy)
  println(Gold().show)
  println((Gold(): Labeled).label)
