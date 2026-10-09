// The Vector's operations across the sizes where its representation changes (a tail alone, a
// trie of one, two and three levels, a front that grew), each checked against a List built the
// same way, and the printed forms.
def same(label: String, v: Vector[Int], expected: List[Int]): Unit =
  val ok = v.length == expected.length && v.toList == expected && v.iterator.toList == expected && {
    var i = 0
    var agree = true
    var rest = expected
    while agree && i < v.length do
      agree = v(i) == rest.head
      rest = rest.tail
      i += 1
    agree
  }
  println(label + ": " + (if ok then "ok " + v.length else "MISMATCH " + v.take(40) + " vs " + expected.take(40)))

@main def main(): Unit =
  for n <- List(0, 1, 31, 32, 33, 64, 65, 1023, 1024, 1025, 1500, 33000) do
    var v = Vector.empty[Int]
    var i = 0
    while i < n do
      v = v :+ i
      i += 1
    val model = List.range(0, n)
    same("appended " + n, v, model)
    same("updated " + n, { var u = v; var j = 0; while j < n do { u = u.updated(j, j * 2); j += 3 }; u }, model.zipWithIndex.map((x, j) => if j % 3 == 0 then x * 2 else x))
    same("prepended " + n, { var p = v; var j = 0; while j < 70 do { p = -j +: p; j += 1 }; p }, List.range(-69, 1) ::: model)
    same("take " + n, v.take(n / 3), model.take(n / 3))
    same("drop " + n, v.drop(n / 3), model.drop(n / 3))
    same("slice " + n, v.slice(n / 4, n - n / 4), model.slice(n / 4, n - n / 4))
    same("drop then append " + n, v.drop(n / 2) :+ 7 :+ 8, model.drop(n / 2) ::: List(7, 8))
    same("take then append " + n, v.take(n / 2) :+ 7 :+ 8, model.take(n / 2) ::: List(7, 8))
    same("map " + n, v.map(_ + 1), model.map(_ + 1))
    same("reverse " + n, v.reverse, model.reverse)
    same("filter " + n, v.filter(_ % 2 == 0), model.filter(_ % 2 == 0))
    println("fold " + n + ": " + v.foldLeft(0L)(_ + _) + " " + v.sum + " " + v.headOption + " " + v.lastOption + " " + v.indexOf(n / 2) + " " + v.contains(n - 1))
  // A queue consumed from the front while fed at the back, and a stack built at the front.
  var q = Vector.tabulate(100)(i => i)
  var taken = 0L
  var k = 0
  while k < 5000 do
    taken += q.head
    q = q.tail :+ k
    k += 1
  println("queue " + taken + " " + q.length + " " + q.head + " " + q.last)
  var s = Vector.empty[Int]
  k = 0
  while k < 3000 do
    s = k +: s
    k += 1
  println("stack " + s.length + " " + s.head + " " + s(1000) + " " + s.last + " " + s.drop(2998))
  // Old versions are unchanged by the updates made from them.
  val base = Vector.tabulate(200)(i => i)
  val branches = List.range(0, 200).map(i => base.updated(i, -1))
  println("branches " + branches.forall(b => b.count(_ == -1) == 1) + " " + base.count(_ == -1) + " " + (base :+ 1).length + " " + (base :+ 2).last + " " + (base :+ 1).last)
  println(Vector(1, 2, 3).toString + " " + Vector.empty[Int] + " " + Vector(1, 2, 3).updated(1, 9) + " " + (0 +: Vector(1)) + " " + Vector.tabulate(40)(i => i).drop(35) + " " + Vector.tabulate(40)(i => i).take(3))
  println(Vector.tabulate(100)(i => i).grouped(30).map(_.length).toList.toString + " " + Vector.tabulate(70)(i => i).sliding(2, 33).toList + " " + Vector.tabulate(50)(i => i).zipWithIndex.last + " " + Vector.tabulate(50)(i => i).splitAt(40)._2)
  try println(Vector(1, 2).apply(5)) catch case e: IndexOutOfBoundsException => println("caught " + e.getMessage)
  try println(Vector.tabulate(100)(i => i).updated(100, 1)) catch case e: IndexOutOfBoundsException => println("caught " + e.getMessage)
