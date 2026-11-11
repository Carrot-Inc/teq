object Main:
  private var order = ""
  def step(label: String, value: Int): Int =
    order += label
    value
  def keep(value: Int): Int = value
  def pair(first: Int, second: Int = 7): Int = first * 10 + second
  def main(args: Array[String]): Unit =
    val result = pair(
      second = keep({ val n = step("s", 2); n }),
      first = pair(second = { val n = step("i", 3); n }, first = step("f", 1)))
    println(result)
    println(order)
    val mapped = List(1, 2).map((n: Int) => pair(second = keep({ val m = n + 1; m }), first = n))
    println(mapped.mkString(","))
