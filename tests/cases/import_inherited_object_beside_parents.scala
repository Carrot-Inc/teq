// An import of an object nested in a class, read through the object that extends the class, does
// not complete that object while the class's own signature is resolved: whether the imported value
// has a type named `Throwable` is asked of the value's type, the object's parents being in progress
// (scala3's `tests/run/i11914`).
class Wrapper[E <: Throwable]:
  object syntax:
    def show(x: Int): String = "x" + x
  def name: String = "wrapper"

object throwables extends Wrapper[Throwable]
import throwables.syntax.*

@main def run(): Unit =
  println(show(1))
  println(throwables.name)
