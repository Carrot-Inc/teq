// A case class named as a method of its object, whose synthetic companion is a term member beside the method
// (dotty enters the module val; `apply(s, s * 2)` resolves to it, the method taking one argument), as
// scalacheck's `object Seed` declares `private case class apply` beside `def apply(s: Long)`: the companion
// `Desugar.classDef` makes and the method are alternatives of one name, as the jar's pickled companion and
// method are (tests/cases/scalacheck_seed_companion.scala).
sealed abstract class Seed:
  def a: Long
  def next: Seed = Seed.apply(a + 1, a + 2)
object Seed:
  private case class apply(a: Long, b: Long) extends Seed
  def apply(s: Long): Seed = apply(s, s * 2)
@main def run(): Unit =
  println(Seed(3L).next.a)
