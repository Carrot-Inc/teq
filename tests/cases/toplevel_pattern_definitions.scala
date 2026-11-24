// Pattern definitions at the top level of a file are a member's, as dotty's package object makes them one: each
// kind desugared to its binders, the initialisers run once, in the file's order, when the file's definitions are
// first read; a binderless one still runs its initialiser, a lazy one at its first read. scalac prints the lines of
// the .expected file.
case class P(a: Int, b: Int)

def f(): Unit = println("eval f")

val (t1, t2) = { println("eval tuple"); (1, 2) }
val Some(s) = { println("eval some"); Some(3) }
val P(m, n) = { println("eval case class"); P(4, 5) }
val (P(q, _), r) = { println("eval nested"); (P(6, 7), 8) }
val (i: Int, j) = { println("eval typed"); (9, "j") }
val p @ P(_, w) = { println("eval alias"); P(10, 11) }
val Some(_) = { println("eval binderless"); Some(12) }
val _ = { println("eval underscore"); 13 }
val () = f()
var Some(v) = { println("eval var"); Some(14) }
lazy val Some(z) = { println("eval lazy"); Some(15) }

object Main:
  def main(args: Array[String]): Unit =
    println("main")
    println((t1, t2, s, m, n, q, r, i, j, p, w))
    println(v)
    v = 16
    println(v)
    println(z)
    println(z)
