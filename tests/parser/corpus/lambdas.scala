case class Item(name: String, qty: Int)

@main def run(): Unit =
  val pairs = List((1, "one"), (2, "two"), (3, "three"))
  val described = pairs.map:
    case (n, word) => s"$n=$word"
  println(described)

  val total = pairs.foldLeft(0):
    case (acc, (n, _)) => acc + n
  println(total)

  val items = List(Item("a", 2), Item("b", 0), Item("c", 5))
  val labels = items.map:
    case Item(name, 0) => s"$name: none"
    case Item(name, q) if q > 3 => s"$name: many"
    case Item(name, q) => s"$name: $q"
  println(labels)

  items.foreach: item =>
    val doubled = item.qty * 2
    println(s"${item.name} -> $doubled")

  val opts = List(Some(1), None, Some(3))
  val flat = opts.flatMap:
    case Some(v) => List(v)
    case None => Nil
  println(flat)

  val add: (Int, Int) => Int = _ + _
  val inc = add(1, _)
  println(inc(41))
  val fs = List[Int => Int](_ + 1, _ * 2, x => x * x)
  println(fs.map(f => f(5)))
  val curried = (a: Int) => (b: Int) => a * b
  println(curried(6)(7))
  def applyTwice(f: Int => Int, x: Int): Int = f(f(x))
  println(applyTwice(_ + 3, 10))
  def show(x: Int): String = s"<$x>"
  println(List(1, 2).map(show))
  println(List("a", "bb").map(_.length))
  println(List(1, 2, 3).map(_.toString).mkString("+"))
