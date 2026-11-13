// A class nested in a class is inherited and named in a subclass; one nested in an object is named
// through the object.
class Outer:
  class In(val n: Int)
object Holder:
  class Kept(val n: Int)
class Sub extends Outer:
  def mk: In = new In(1)

@main def run(): Unit =
  println(new Sub().mk.n + new Holder.Kept(2).n)
