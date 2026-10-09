// expect: `do <body> while <cond>` is no longer supported, use `while <body> ; <cond> do ()` instead
// expect: expected 'then', found identifier
// expect: `yield` or `do` expected

object E:
  def f(xs: List[Int]): Unit =
    var i = 0
    val z = do i += 1 while (i < 3)
    val y = if i > 0 i else 1
  def g(xs: List[Int]): Unit =
    for x <- xs
      println(x)
