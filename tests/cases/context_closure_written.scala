// A context function literal as written (`isContextualClosure`) passed or bound where another type
// is expected stays the function, whatever givens are in scope: its body does not run. An
// ascription's closure is no such literal: it is applied to the given in scope, its value computed
// once (dotty's `adaptNoArgsOther`).
var count = 0
def use(x: Any): Unit = println(if x.isInstanceOf[Int] then s"value $x" else "a function")
def one: Int = { count += 1; 1 }

object WithGiven:
  given Int = 7
  def run(): Unit =
    use((x: Int) ?=> { println("body"); x + 1 })
    use(((x: Int) ?=> { println("body"); x + 1 }))
    val a: Any = (x: Int) ?=> { println("body"); x + 1 }
    use(a)
    use((one: (Int ?=> Int)))
    println(count)
    val b: Any = (summon[Int] + one: (Int ?=> Int))
    println(b)
    println(count)

object WithoutGiven:
  def run(): Unit =
    use((x: Int) ?=> { println("body"); x + 1 })
    val c: Any = (s: String) ?=> s.length
    use(c)

@main def main(): Unit =
  WithGiven.run()
  WithoutGiven.run()
