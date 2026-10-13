// The implementation of a deferred given is a declaration of the class that inherits it (dotty's
// `Typer.implementDeferredGivens`: `dcl.copy(..).entered`, 3420-3423), beside a method of its name
// with other parameters: reflection's declared fields list it, and a subclass inherits it.
import scala.compiletime.deferred
trait T:
  given x: Int = deferred
class C(using n: Int) extends T:
  def x(s: String) = s
class D extends C(using 3)
@main def run(): Unit =
  println(M.fields[C])
  println(M.fields[D])
  val d = new D
  println(s"${d.x} ${d.x("d")} ${(d: T).x}")
