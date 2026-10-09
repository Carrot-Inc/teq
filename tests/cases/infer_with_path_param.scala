// A parameter whose type is a path of an object (zio's `trace: Trace`, `Tracer.instance.Type`)
// names no parameter: the arguments' types stay open until the application solves them, so
// `LowerE` is still taken from below.
trait TracerApi:
  type Type
  def empty: Type

object Tracer:
  val instance: TracerApi = new TracerApi:
    type Type = String
    def empty: String = "trace"

type Trace = Tracer.instance.Type
given Trace = Tracer.instance.empty

class Aspect[+LowerE, -UpperE]
class Eff[+E](val tag: String):
  def @@[LowerE >: E, UpperE >: LowerE](aspect: => Aspect[LowerE, UpperE])(using trace: Tracer.instance.Type): Eff[LowerE] =
    Eff(tag + "@" + trace.toString)

class Counter(val name: String) extends Aspect[Nothing, Any]
def counter(name: String): Counter = Counter(name)

@main def run(): Unit =
  val r = Eff[Nothing]("u") @@ counter("pushes")
  val s: Eff[Nothing] = r
  println(s.tag)
