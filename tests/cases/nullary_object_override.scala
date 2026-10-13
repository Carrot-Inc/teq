// A class member overriding `Any.toString` matches it loosely (`matchNullaryLoosely` of the
// overridden symbol), so `O.toString()` calls the parameterless override; a synthesized case
// class member implements a trait's deferred one.
object O:
  override def toString: String = "O"
trait T:
  override def toString: String
case class C(n: Int) extends T

@main def run(): Unit =
  println(O.toString())
  println(C(1))
