// Two bodies whose calls are expanded, the second refused by its macro, and a last inline method whose body
// calls an array's member: the refusal is reported whatever the count of workers.
def bodyOne(): Int = Checked.positive(1)
def bodyTwo(): Int = Checked.positive(-2)
object Main:
  def main(args: Array[String]): Unit =
    println(bodyOne() + bodyTwo())
inline def last(a: Array[Int]): Int = a.length
