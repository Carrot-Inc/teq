// A collection is a function wherever a function type is expected, inside another value too (scala-library's
// `MapOps extends PartialFunction`, `Seq` and `Set`): stored in a list, a generic result, a variable, an array,
// an option, passed to a template and a helper that call it, and called through a partial function's members.
object Main:
  def pick[A](xs: List[A]): A = xs.head

  def twice(f: Int => String, x: Int): String = f(x) + f(x)

  def main(args: Array[String]): Unit =
    val maps: List[Map[Int, String]] = List(Map(1 -> "one"))
    val fs: List[Int => String] = maps
    println(fs.head(1))
    println(pick(maps)(1))
    val g: Int => String = pick[Int => String](fs)
    println(g(1))
    var h: Int => String = (i: Int) => "lambda " + i
    println(h(2))
    h = Map(2 -> "two")
    println(h(2))
    println(twice(h, 2))
    val seqs: Vector[Int => String] = Vector(List("a", "b", "c"), Vector("x", "y"))
    println(seqs.map(f => f(1)))
    val sets: Option[String => Boolean] = Some(Set("s"))
    println(sets.map(p => (p("s"), p("t"))))
    val arr: Array[Int => Int] = Array(Seq(10, 20, 30), (i: Int) => i * 2)
    println(arr.map(f => f(2)).toList)
    println(Array(0, 2, 1).map(Seq("p", "q", "r")).toList)
    println(Array.tabulate(3)(Vector(7, 8, 9)).toList)
    println(List(1, 2, 3).map(Map(1 -> "a", 2 -> "b", 3 -> "c")))
    println(List("a", "z").filter(Set("a")))
    val pfs: List[PartialFunction[Int, String]] = List[PartialFunction[Int, String]](Map(1 -> "m"), { case 2 => "case" })
    println(pfs.map(pf => (pf.isDefinedAt(1), pf.lift(1), pf.lift(2))))
    println(pfs.map(pf => pf.applyOrElse(3, (n: Int) => "default " + n)))
    println(List(1, 2, 3).collect(pfs.head.orElse(pfs(1))))
    val composed = fs.head.andThen(_.toUpperCase)
    println(composed(1))
    val any: Any = Map(1 -> "cast")
    println(any.asInstanceOf[Int => String](1))
    any match
      case f: Function1[?, ?] => println("a function: " + f.asInstanceOf[Int => String](1))
      case _ => println("no function")
    println(List(1, 2).map(maps.head.withDefaultValue("none")))
