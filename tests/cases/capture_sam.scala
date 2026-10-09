// A function where a trait with a single abstract method is expected is held in a local named
// after the expression's offset in the file, which the function reads past to the definition.
@FunctionalInterface
trait Op:
  def apply(x: Int): Int

def `sam$353`(): Int = 7
def m(a: Int)(b: Int): Int = a * 10 + b

@main def main(): Unit =
  val op: Op = m(`sam$353`())
  println(op(1))
