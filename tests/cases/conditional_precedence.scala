// A conditional where JavaScript's precedence differs from the tree's: as the condition of
// another, whose condition then needs its parentheses, and beside operators, receivers, callees,
// arguments, elements and interpolations, and an arrow function beside an operator; `t` counts
// the conditions evaluated.
object Main:
  var n = 0
  def t(b: Boolean): Boolean = { n += 1; b }
  def main(args: Array[String]): Unit =
    val v = args.length == 0
    val w = !v
    println(if (if v then true else false) then "a" else "b")
    println(if (if w then true else false) then "a" else "b")
    println((if (if v then false else true) then "a" else "b") + "!")
    println((if v then "x" else "yy").length)
    println((if w then "x" else "yy").length)
    println((if v then 1 else 2) + 10)
    println(10 + (if v then 1 else 2))
    println(-(if v then 1 else 2))
    println(!(if v then false else true))
    println((if v then 3 else 4) * (if w then 5 else 6))
    println((if v then List(1) else List(2, 3)).size)
    println((if v then "a" else "b") == "a")
    println((if v then 1 else 2) == 1)
    val f = if v then (x: Int) => x + 1 else (x: Int) => x + 2
    println(f(1))
    println((if v then (x: Int) => x + 1 else (x: Int) => x * 2)(5))
    val arr = Array(if v then 1 else 2, if w then 3 else 4)
    println(arr.toList)
    println(List(if v then "p" else "q").head)
    println(s"${if v then "s" else "t"}!")
    println(if v then (if w then "1" else "2") else "3")
    println(if (if v then w else v) then "c1" else "c2")
    println(if (if w then v else w) || v then "d1" else "d2")
    println((if t(v) then 1 else 2) + (if t(w) then 10 else 20))
    println(n)
    var z = 0
    z = if v then 5 else 6
    println(z)
    println((if v then Some(1) else None).getOrElse(0))
    println((if v then 2 else 3).toString)
    println(((x: Int) => if x > 0 then "pos" else "neg")(1))
    println((if v then Seq(1, 2) else Seq(3)).map(_ + 1))
    println((if (v && w) || v then "e1" else "e2"))
    println(if v && (if w then true else v) then "f1" else "f2")
    println(if ((v || w) && !(if v then true else false)) "g1" else "g2")
    val o: Option[Int] = if v then Some(1) else None
    println(o.map(x => if x > 0 then x else -x))
    println(Seq(1,2,3).map(x => if x > 1 then x * 2 else x).sum)
    println(if (if t(v) then t(w) else t(v)) then (if t(w) then "h1" else "h2") else "h3")
    println(n)
    println((if (if v then 1 > 0 else 1 < 0) then List(1, 2) else Nil).length)
    val g = (x: Boolean) => if (if x then !x else x) then "i1" else "i2"
    println(g(true) + g(false))
    println(Seq(true, false).map(x => if (if x then x else !x) then 1 else 0))
    println(-(if (if v then w else v) then 1 else 2))
    println("" + (if (if w then v else w) then 'x' else 'y') + (if v then 'z' else 'q'))
    val id: Int => Int = x => x
    println(((x: Int) => x) eq id)
    println(((x: Int) => x + 1) != id)
