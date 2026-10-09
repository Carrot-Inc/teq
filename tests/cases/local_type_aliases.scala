trait Box:
  type Value
  def value: Value
object Main:
  def show(b: Box): String =
    type T = b.Value & AnyRef
    val v = b.value.asInstanceOf[T]
    def same(x: T, y: T): Boolean = x eq y
    s"${v.toString} ${same(v, v)}"
  def pick[A](a: A): String =
    type A0 = A & AnyRef
    val xs: List[A0] = List(a.asInstanceOf[A0])
    xs.head.toString
  def pairs(n: Int): List[String] =
    type P[X] = (X, X)
    type Named = P[String]
    val ps: List[P[Int]] = List.tabulate(n)(i => (i, i * i))
    val named: Named = ("a", "b")
    ps.map((a, b) => s"$a->$b") :+ named.toString
  def main(args: Array[String]): Unit =
    println(show(new Box { type Value = String; def value = "hi" }))
    println(pick(List(1)))
    println(pairs(3))
