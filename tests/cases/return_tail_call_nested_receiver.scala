// A jump on another instance goes round with the receiver and the arguments it evaluated, each evaluated before
// the loop's receiver or a parameter changes, as dotty's TailRec evaluates them into temporaries: a `return`
// inside an argument or the receiver is a jump of its own with the receiver it found.
final class Node(val next: Node, val tag: Int) {
  final def first(n: Int, m: Int): Int = { if n > 0 then return next.first(if n == 1 then return first(0, m + tag) else n - 1, m + tag); m * 10 + tag }
  final def second(n: Int, m: Int): Int = { if n > 0 then return next.second(n - 1, if n == 1 then return second(0, m + tag) else m + tag); m * 10 + tag }
  final def deep(n: Int, m: Int): Int = { if n > 0 then return next.deep(n - 1, m + (if n == 1 then (if m >= 0 then return deep(0, m + 100 + tag) else 0) else tag)); m * 10 + tag }
  final def receiver(n: Int, m: Int): Int = { if n > 0 then return (if n == 1 then return receiver(0, m + 50 + tag) else next).receiver(n - 1, m + tag); m * 10 + tag }
}
final class Cell(val id: Int) {
  var next: Cell = this
  final def f(n: Int): Int = {
    if n > 0 then return next.f(if n == 1 then return f(0) else n - 1)
    id
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val chain = new Node(new Node(new Node(null, 3), 2), 1)
    println(List(chain.first(2, 0), chain.second(2, 0), chain.deep(2, 0), chain.receiver(2, 0)).mkString(" "))
    val a = new Cell(1)
    a.next = new Cell(2)
    println(a.f(1))
  }
}
