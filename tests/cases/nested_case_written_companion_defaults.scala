// A case class nested in a class beside its written companion, its defaults in two clauses reading the
// enclosing instance and the clause before theirs: every default of `o.Item(..)(..)` is selected on the
// companion of the creation's prefix (`Applications.findDefaultGetter`), the one lifted out of the
// construction for the later clause's default too (`liftFun`), and the receiver and the arguments are
// evaluated once, in order.
class Outer(val n: Int) {
  var made = 0
  case class Item(a: Int, b: Int = n + 1)(val c: Int = a + b + n) {
    made += 1
  }
  object Item {
    def tag = "item"
    def first: Item = Item(0)()
  }
  def inside: Item = Item(5)()
}

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer(10)
    def recv = { println("receiver"); o }
    def one = { println("one"); 1 }
    val i = recv.Item(one)()
    println(i.toString + " " + i.c)
    val j = new o.Item(2)(3)
    println(j.toString + " " + j.c + " " + o.made + " " + o.Item.tag)
    println(o.Item.first.c)
    println(o.Item(4, 5)().c)
    println(o.inside.c)
    val p = new Outer(20)
    println(s"${p.Item(1)().c} ${o.made} ${p.made}")
  }
}
