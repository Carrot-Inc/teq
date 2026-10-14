// A base its parents give at invariant arguments that differ is their instances' intersection (dotty's `glbArgs`
// declines the joint instance and `glb` keeps the `AndType`): `Both` is a `Holder[String]` and a `Holder[Int]`, and
// a member read through it takes each parameter's arguments met by position, `value` a `String & Int` and `put`
// either. No class may be such a `Both` (scalac refuses one that has conflicting base types), so the reads are
// typed and never run.
trait Holder[T]:
  def value: T
  def put(a: T): String
trait A extends Holder[String]
trait B extends Holder[Int]
trait Both extends A with B

def first(b: Both): String = b.value
def second(b: Both): Int = b.value
def asA(b: Both): Holder[String] = b
def asB(b: Both): Holder[Int] = b
def putEither(b: Both): String = b.put("x") + b.put(1)

@main def main(): Unit =
  val reads: List[Both => Any] = List(first, second, asA, asB, putEither)
  println(reads.size)
