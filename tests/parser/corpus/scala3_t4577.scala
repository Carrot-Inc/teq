// Adapted from scala3 tests/run/t4577.scala (Apache-2.0, see tests/scala3/README.md); replaced: `if (c)` with `if c then`, the asserts print their results.
object Test {
  val bippy    = new Symbol("bippy")
  val imposter = new Symbol("bippy")
  val notBippy = new Symbol("not-bippy")
  val syms = List(bippy, imposter, notBippy)

  // the equals method should only be used for case `bippy`,
  // for the singleton type pattern, case _: bippy.type, the spec mandates `bippy eq _` as the test
  class Symbol(val name: String) {
    override def equals(other: Any) = other match {
      case x: Symbol  => name == x.name
      case _          => false
    }
    override def toString = name
  }

  def f(s: Symbol) = s match {
    case _: bippy.type  => true
    case _              => false
  }
  def fDirect(s: Symbol) = bippy eq s

  def g(s: Symbol) = s match {
    case _: bippy.type => 1
    case `bippy`       => 2
    case _             => 3
  }
  def gDirect(s: Symbol) = if bippy eq s then 1 else if bippy == s then 2 else 3

  def main(args: Array[String]): Unit = {
    println(syms.map(f))
    println(syms.forall(s => f(s) == fDirect(s)))
    println(syms.map(g))
    println(syms.forall(s => g(s) == gDirect(s)))
  }
}
