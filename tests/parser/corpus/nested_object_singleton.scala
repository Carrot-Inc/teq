// The singleton type of an object nested in a class names the enclosing instance's object:
// `a.R.type` takes `a.R`, and inside the class `R.type` takes `R`.
class O:
  object R:
    override def toString = "R"
  val mine: R.type = R
  def same(r: R.type): Boolean = r eq R

@main def run(): Unit =
  val a = O()
  val b = O()
  val q: a.R.type = a.R
  val f: a.R.type => String = r => r.toString
  println(f(q))
  println(a.same(a.mine))
  println(a.mine eq b.mine)
