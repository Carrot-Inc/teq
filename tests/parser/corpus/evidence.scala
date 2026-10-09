// `A <:< B` and `A =:= B` evidence is supplied by the compiler where `A` conforms to `B` (is the
// same as `B`); the value is the identity function and is usable as an `A => B`.
final case class Box[A](value: A):
  def flatten[B](using ev: A <:< Option[B]): Option[B] = ev(value)
  def same[B](using ev: A =:= B): B = ev(value)
  def widened[B](using ev: A <:< B): List[B] = List(value).map(ev)

def widen[A, B](a: A)(using ev: A <:< B): B = ev(a)
def onlyStrings[A](a: A)(using ev: A =:= String): Int = ev(a).length
sealed trait Animal
final case class Dog(name: String) extends Animal

@main def main(): Unit =
  println(Box(Option(1)).flatten)
  println(Box(Some(2)).flatten)
  println(Box(3).same + 1)
  val s: Any = widen[String, Any]("x")
  println(s)
  println(onlyStrings("abcd"))
  val animals: List[Animal] = Box(Dog("rex")).widened
  println(animals)
  val ev = summon[Dog <:< Animal]
  println(ev(Dog("fido")))
  println(ev.andThen(a => a.toString.length)(Dog("fi")))
  val f: Dog => Animal = ev
  println(f(Dog("g")))
  println(summon[Int =:= Int](5))
  println(summon[List[Dog] <:< Seq[Animal]](List(Dog("a"))).length)
