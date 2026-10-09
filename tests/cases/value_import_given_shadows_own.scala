// A given imported from a value is read on that value, also inside a class whose own given of
// the same name the import shadows.
class C(val n: Int):
  given g: Int = n
  def pick(c: C): Int =
    import c.given
    summon[Int]

@main def run(): Unit = println(new C(1).pick(new C(2)))
