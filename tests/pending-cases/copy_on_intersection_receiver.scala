// A case class's `copy` selected on an intersection receiver (`CC[Int] & K`, `CC[Int] & CC[String]`): scalac 3.8.4
// finds the member on the receiver's part and types the call, `copy`'s default getter seen from the receiver (the two
// instances' `CC[Int & String]`). teq reports "value copy is not a member of CC[Int] & K" (master the same). The
// closure adversary of the pass on 97f0487ac (round 24).
case class CC[A](a: A)
trait K
def t(x: CC[Int] & K): CC[Int] = x.copy()
def u(x: CC[Int] & Serializable): CC[Int] = x.copy(a = 2)
def v(x: CC[Int] & CC[String]): CC[Int & String] = x.copy()

@main def run(): Unit =
  println(t(new CC(1) with K).a + u(CC(1)).a)
