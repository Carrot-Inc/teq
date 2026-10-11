// The members of an intersection reached through a receiver that is not written as one: a type
// parameter's bound (an overloaded name's alternatives met there), a self type (an unqualified
// call), and an intersection of eleven instances, every one met (`ClassDenotation.baseTypeOf`).
trait R[K <: String]:
  def get(k: K): Any
  def get(n: Int): Int
class Store extends R[String]:
  def get(k: String): Any = "got " + k
  def get(n: Int): Int = n + 1

// An invariant self type no class conforms to is typed alone; a contravariant one's instances
// meet as the union of their arguments.
trait S[K <: String]:
  def one(k: K): String
trait T:
  self: S["k"] & S["v"] =>
  def a: String = one("k")
  def b: String = one("v")
trait P[-K <: String]:
  def two(k: K): String
class PS extends P[String]:
  def two(k: String): String = "two " + k
trait Q:
  self: P["k"] & P["v"] =>
  def a: String = two("k")
  def b: String = two("v")
class QS extends PS with Q

trait N[K]:
  def at(k: K): String
class NS extends N[Any]:
  def at(k: Any): String = "at " + k

def bounded[X <: R["k"] & R["v"]](r: X): String = r.get("k").toString + ", " + r.get("v") + ", " + r.get(41)
def many(r: N[0] & N[1] & N[2] & N[3] & N[4] & N[5] & N[6] & N[7] & N[8] & N[9] & N[10]): String = r.at(1) + ", " + r.at(10)

@main def main(): Unit =
  println(bounded((new Store).asInstanceOf[R["k"] & R["v"]]))
  val q = new QS
  println(q.a + ", " + q.b)
  println(many((new NS).asInstanceOf[N[0] & N[1] & N[2] & N[3] & N[4] & N[5] & N[6] & N[7] & N[8] & N[9] & N[10]]))
