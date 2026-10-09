// A statement begun on the line of a `{` is ended by that line's break as by every later one
// (dotty's `Scanners.handleNewLine`, the first line giving the braces their width,
// `proposeKnownWidth`; `Parsers.blockStatSeq` then takes the statements apart), but where the next
// line goes on with an infix operator, `else` or a `.`.
def f(x: Int): Int = { val k = x
  if k > 0 then
    k
  else -k
}

class C { val x = 1; def y = x
  def z = y + 1
}

@main def run(): Unit =
  val g: Int => Int = s => { val k = 1
    s match
      case 1 => s + k
      case _ => 0
  }
  println(g(1))
  println(g(2))
  val r = { val k = 1
    k + 1
  }
  println(r)
  val s = { println("a")
    println("b"); 3 }
  println(s)
  println(f(-2))
  println(C().z)
  List(1, 2).foreach { x => val y = x * 2
    println(y)
  }
  val h = (n: Int) => { val a = n
    val b = a + 1
    a * b }
  println(h(3))
  val q = { val one = 1
    { val z = one
      z + 1 }
  }
  println(q)
  val sum = { r
    + 2 }
  println(sum)
  val sign = { if r > 0 then "pos"
    else "neg" }
  println(sign)
  val sorted = { List(3, 1, 2)
    .sorted
    .mkString(",") }
  println(sorted)
  val named = List(1, 2, 3).map { case 1 => "one"
                                  case 2 => "two"
                                  case _ => "many" }
  println(named)
