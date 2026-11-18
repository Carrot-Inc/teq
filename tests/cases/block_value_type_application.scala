// A type argument list after an expression written in braces applies to the value of the block: its `apply`,
// not a method the block names. scalac prints (List(1, 2),ap,ap).
class P { def apply[T]: String = "ap" }
val poly = new P
def make[A]: P = new P
def companion: Any = { List }[Int](1, 2)
def value: Any = { poly }[Int]
def result: Any = { make }[Int]
@main def main(): Unit = println((companion, value, result))
