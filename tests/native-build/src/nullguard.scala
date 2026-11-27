class Box(val n: Int) { def v: Int = n }
class Box2(m: Int) extends Box(m) { override def v: Int = n + 100 }

def twice(box: Box): Unit =
  try println(box.v) catch case _: NullPointerException => println("caught in the first try")
  try println(box.v) catch case _: NullPointerException => println("caught in the second try")

def afterBranch(box: Box, c: Boolean, d: Boolean): Int =
  if c then println(box.v)
  if d then println("after the branch")
  box.v

@main def nullguard(): Unit =
  twice(null)
  twice(Box2(1))
  try println(afterBranch(null, false, true)) catch case _: NullPointerException => println("caught after the branch")
  println(afterBranch(Box(2), true, true))
