@main def run(): Unit =
  val first = new Enclosing(10)
  val second = new Enclosing(20)
  val left = new first.Inner(5)
  val right = new second.Inner(6)
  println(left.add(2))
  println(right.add(3))
  println(left.add(4))
  println(left.both)
  println(right.both)
  println(left.nested(2))
  println(right.local(3))
  val deep = new left.Deeper(7)
  println(deep.total)
  var calls = 0
  def receiver(): first.Inner =
    calls += 1
    left
  println(receiver().both)
  println(calls)
