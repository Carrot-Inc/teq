// A local `inline given`, transparent or not, is an inline method (dotty's `Parsers.givenDef`: a
// `DefDef` where the alias is inline), its right-hand side expanded at each use.
@main def run(): Unit =
  var n = 0
  transparent inline given x: Int = { n += 1; n }
  println(summon[Int]); println(summon[Int])
  var m = 0
  inline given y: Long = { m += 1; m.toLong }
  println(summon[Long]); println(summon[Long]); println(m)
  def twice(using k: Int): Int = k * 2
  println(twice); println(n)
