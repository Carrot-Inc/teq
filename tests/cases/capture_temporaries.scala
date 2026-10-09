// The typer's temporaries are locals of the output like any other: one named like a definition
// its scope reads is named apart from it (docs/TARGETS.md, "The JavaScript output"). A named
// argument hoisted out of order, a case lambda's parameters and a contextual function's
// parameter meet a definition named as they are; a tupled argument and an eta-expansion do not.
def `a$0`(): Int = 7
def `c$0`(): Int = 7
def `contextual$1`(): Int = 7
def `eta$0`(): Int = 7
def `h$0`(): Int = 7

def sum(t: (Int, Int)): Int = t._1 + t._2
def twice(x: => Int): Int = x + x
def f(a: Int, b: Int): Int = a * 10 + b

def tupled(): Int =
  val g: ((Int, Int)) => Int = sum
  val h: (Int, Int) => Int = (x, y) => g((x, y)) + `a$0`()
  List((1, 2)).map(sum).sum + h(1, 2)

def caseLambda(): Int =
  val g: (Int, Int) => Int = { case (a, b) => `c$0`() + a + b }
  g(1, 2)

def contextual(): Int =
  val g: Int ?=> Int = summon[Int] + `contextual$1`()
  g(using 1)

def byName(): Int =
  val g: (=> Int) => Int = twice
  g(`eta$0`())

def namedArguments(): Int = f(b = `h$0`(), a = { println("a"); 2 })

@main def main(): Unit =
  println(tupled())
  println(caseLambda())
  println(contextual())
  println(byName())
  println(namedArguments())
