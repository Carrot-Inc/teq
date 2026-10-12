// A builtin operator selected with no argument where a function is expected is eta-expanded as the
// primitive class's method is (dotc's etaExpand of `Int.<`), ahead of an implicit conversion's member
// of the name (`any2stringadd`'s `+`); an unstable receiver is evaluated once.
@main def run(): Unit =
  val xs = List(3, -1, 4, 0)
  println(xs.filter(0.<))
  println(xs.map(1.+).sum)
  println(xs.exists(4.==))
  println("a-b".takeWhile('-'.!=))
  var calls = 0
  def next(): Int = { calls += 1; 10 }
  val add: Int => Int = next().+
  println(add(1) + add(2))
  println(calls)
  val longer: Int => Long = 5L.*
  println(longer(3))
  println(List(true, false).map(true.&))
