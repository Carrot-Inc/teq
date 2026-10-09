// jars: scala-library traitfields-lib
// A trait's setter stores into the class's field of the val's name, the trait's or an overriding
// val's, as scalac's `Memoize` makes it, so the trait's `$init$` reads the trait's value before the
// class's initialiser sets its own (master read 0), whether a class or a trait further down
// overrides it; a constructor parameter that overrides the val keeps its value, which the trait's
// `$init$` reads. A jar trait (`tfl.Over`) and a program trait alike.
import tfl.Over

trait POver:
  val o: Int = 10
  println("POver init sees " + o)

class JD extends Over:
  override val o: Int = 20
class JP(override val o: Int) extends Over
class PD extends POver:
  override val o: Int = 20
class PP(override val o: Int) extends POver
trait Further extends POver:
  override val o: Int = 40
  println("Further init sees " + o)
class PF extends Further

@main def run(): Unit =
  println(JD().o)
  println(JP(25).o)
  println(PD().o)
  println(PP(25).o)
  println(PF().o)
