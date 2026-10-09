// A for comprehension over an array with a binding and a guard yields an array, each guard run
// as the body after it reaches its element, and the element read before its guard is what `map`
// and `foreach` pass on, whatever the guard writes (`flatMap` reads it again, as scala-library's
// does); `withFilter` alone, then `foreach`, and two guards.
@main def run =
  val xs = Array(1, 2, 3, 4)
  val ys = for
    x <- xs
    y = x * 10
    if y > 15
  yield y + 1
  println(ys.mkString(","))
  println(ys.length)
  xs.withFilter(_ % 2 == 0).foreach(println)
  val zs = for x <- Array(1, 2) if { print("p" + x); true } yield { print("m" + x); x }
  println(" " + zs.length)
  for x <- Array(1, 2, 3, 4) if x > 1 if x < 4 do print(x)
  println()
  println((for x <- Array(1, 2, 3) if x != 2; y <- List(x, x * 10) yield y).mkString(","))
  val w = Array(1, 2)
  println(w.withFilter { x => w(x - 1) = 99; true }.map(identity).mkString(","))
  val v = Array(1, 2)
  v.withFilter { x => v(x - 1) = 77; true }.foreach(x => print(x))
  println()
  val u = Array(1, 2)
  println(u.withFilter { x => u(x - 1) = 55; true }.flatMap(x => List(x)).mkString(","))
