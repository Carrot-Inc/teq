// expect: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: pattern's type Int is more specialized than the right hand side expression's type Any
// expect: warning: pattern's type ::[Int] is more specialized than the right hand side expression's type List[Int]
@main def run(): Unit =
  val xs = List(Option(1), None)
  val ys = for Some(x) <- xs yield x
  println(ys)
  val zs: List[Any] = List(1, "s")
  val ints = for (i: Int) <- zs yield i
  println(ints)
  val h :: t = List(1, 2)
  println(h)
