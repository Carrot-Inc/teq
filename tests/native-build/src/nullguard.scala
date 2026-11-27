class Box(val n: Int) { def v: Int = n }
class Box2(m: Int) extends Box(m) { override def v: Int = n + 100 }

def twice(box: Box): Unit =
  try println(box.v) catch case _: NullPointerException => println("caught in the first try")
  try println(box.v) catch case _: NullPointerException => println("caught in the second try")

def afterBranch(box: Box, c: Boolean, d: Boolean): Int =
  if c then println(box.v)
  if d then println("after the branch")
  box.v

def reorder(p: Box, q: Box, c: Boolean): Unit =
  try
    if c then throw IllegalStateException()
    println(p.v)
  catch
    case _: IllegalStateException =>
      println(p.v)
      try println(q.v) catch case _: IllegalStateException => println("z")

def locked(o: AnyRef): Int = o.synchronized(7)

def lockedInTry(o: AnyRef): Unit =
  try
    o.synchronized(7)
    println("locked")
  catch case _: NullPointerException => println("caught in the try")

def lockedInTryFinally(o: AnyRef): Unit =
  try o.synchronized(println("locked")) finally println("finally")

@main def nullguard(): Unit =
  twice(null)
  twice(Box2(1))
  try println(afterBranch(null, false, true)) catch case _: NullPointerException => println("caught after the branch")
  println(afterBranch(Box(2), true, true))
  reorder(Box2(1), Box2(2), false)
  reorder(Box2(1), Box2(2), true)
  try reorder(null, Box(2), true) catch case _: NullPointerException => println("caught in a catch case")
  try println(locked(null)) catch case _: NullPointerException => println("caught a monitor of null")
  println(locked("m"))
  lockedInTry(null)
  lockedInTry("m")
  try lockedInTryFinally(null) catch case _: NullPointerException => println("caught after the finally")
  lockedInTryFinally("m")
