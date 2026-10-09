// The concrete members of Product on the classes that are products by rule: productIterator
// and canEqual on a case class, a case object, a tuple, an enum case and a value typed Product.
case class P(n: Int, s: String)
case object Solo
enum Color:
  case Red, Green

object Main:
  def main(args: Array[String]): Unit =
    val p = P(1, "x")
    println(p.productIterator.toList)
    println(p.productIterator.mkString("[", ", ", "]"))
    println(Solo.productIterator.toList)
    println((1, "a", true).productIterator.toList)
    println(Color.Red.productIterator.toList)
    val q: Product = p
    println(q.productArity)
    println(q.productElement(1))
    println(q.productPrefix)
    println(q.productIterator.toList)
    println(q.canEqual(P(2, "y")))
    println(q.canEqual((1, 2)))
    println(p.canEqual(null))
    println((1, 2).canEqual((3, 4)))
    println((1, 2).canEqual((1, 2, 3)))
    val ps: List[Product] = List(p, Solo, (1, "a"), Color.Green)
    println(ps.map(_.productIterator.toList.length))
