// Two members of an intersection's parts whose later parameters name their earlier ones
// (`y: x.T`): matched and met with the second's term binders read as the first's
// (`matchingMethodParams`, `infoMeet`), so that `r.f(x)(y)` is one member of the met result.
trait K:
  type T
  def make: T
object KS extends K:
  type T = String
  def make: String = "made"

trait A:
  def f(x: K)(y: x.T): Product
trait B:
  def f(x: K)(y: x.T): Serializable
class AB extends A with B:
  def f(x: K)(y: x.T): Product & Serializable = Some(y)

def met(r: A & B)(x: K)(y: x.T): Product & Serializable = r.f(x)(y)

@main def main(): Unit =
  println(met(new AB)(KS)(KS.make))
