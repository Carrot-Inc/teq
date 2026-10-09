// A given without parameters is a lazy val: it is initialised on first use, in objects and in
// blocks alike, and a plain alias of an object touches the object only then.
trait T:
  def n: String
class TI(val n: String) extends T:
  println("TI(" + n + ")")
object Impl extends T:
  def n = "impl"
  println("init Impl")

object O:
  println("O body start")
  given simple: T = TI("simple")
  given computed: T = { println("computing"); TI("computed") }
  val v = TI("val")
  println("O body end")

object Forward:
  val fromGiven = summon[T].n
  given g: T = TI("g")
  println("fromGiven = " + fromGiven)

object Alias:
  println("Alias start")
  given g: T = Impl
  val other = TI("other")
  println("Alias end")

def useComputed(using t: T) = t.n

@main def run(): Unit =
  println("main")
  println(O.v.n)
  println(O.simple eq O.simple)
  println(useComputed(using O.computed))
  println(useComputed(using O.computed))
  println(O.computed eq O.computed)
  println(Forward.fromGiven)
  println(Alias.other.n)
  println(Alias.g.n)
  println("start")
  given local: T = { println("computing local"); TI("local") }
  println("after given")
  println(summon[T].n)
  println(summon[T] eq summon[T])
