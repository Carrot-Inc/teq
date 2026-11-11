// A file that calls a macro of the run is typed again once the macro can run. Its classes nested in an object
// or a class keep the constructors their context bounds give them: the evidence clause after the written ones.
object Outer:
  class Tag[T: Ordering]:
    def larger(a: T, b: T): T = if summon[Ordering[T]].gt(a, b) then a else b

  abstract class Named[T: Ordering](val name: String):
    val answer: Int = Macros.answer
    def smaller(a: T, b: T): T = if summon[Ordering[T]].lt(a, b) then a else b

  object Ints extends Named[Int]("ints")

class Holder:
  class Inner[T: Ordering](val first: T):
    def max(other: T): T = summon[Ordering[T]].max(first, other)

@main def run(): Unit =
  println(Macros.answer)
  println(new Outer.Tag[Int].larger(3, 7))
  println(Outer.Ints.name + " " + Outer.Ints.answer + " " + Outer.Ints.smaller(3, 7))
  val holder = new Holder
  println(new holder.Inner("b").max("a"))
