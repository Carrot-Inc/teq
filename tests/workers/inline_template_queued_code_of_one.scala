// An inline method's expansion makes strings while the calls are expanded (`codeOf`), beside a last inline
// method whose body calls an array's member: one such call.
inline def first(inline a: List[Int]): String = scala.compiletime.codeOf(a)
def bodyOne(a: List[Int]) = first(a)
def bodyTwo(b: List[Int]) = first(b)
object Main:
  def main(args: Array[String]): Unit =
    println(bodyOne(List(1, 2)) + bodyTwo(List(3)))
inline def last(a: Array[Int]) = a.length
