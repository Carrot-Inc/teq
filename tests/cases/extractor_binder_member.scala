// A member both parts of an intersection declare takes the narrower of the two result types,
// as scalac's member of an intersection has their meet: the binder `b: B & A` reads `value` as
// `A`'s `String`, which chooses `pick(String)`.
trait A { def value: String }
trait B { def value: Any }
class C extends A with B { def value: String = "ok" }
object E { def unapply(x: B): Option[Int] = Some(1) }
def pick(x: Any): String = "any"
def pick(x: String): String = "string"
def f(x: A): String = x match
  case b @ E(_) => pick(b.value)
  case _ => "none"
def g(x: B & A): String = pick(x.value)
@main def run(): Unit =
  println(f(new C))
  println(g(new C))
