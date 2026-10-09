package avu

import avl.*

class Store:
  var cell = 0

class OverCell extends Over:
  var w: Int = 0

class GenCell extends Handle[Int]:
  var current: Int = 0
  def current(n: Int): Int = current + n

object Use:
  def main(args: Array[String]): Unit =
    val st = Store()
    val a: AbsVar = new AbsVar { var x = 1 }
    a.x = 2; a.x += 1; println(a.x)
    val d: AbsVar = new AbsVar { def x = st.cell; def x_=(v: Int) = st.cell = v * 2 }
    d.x = 5; Lib.bump(d); println(s"${d.x} ${st.cell}")
    println(s"${Lib.assign(a, 7)} ${Lib.assign(d, 8)}")
    val p: DefPair = new DefPair { var y = 3 }
    p.y = 4; p.y += 1; println(p.y)
    val h: Handle[Int] = new Handle[Int] { var current = 0 }
    println(s"${Lib.put(h, 9)} ${Lib.put(new Handle[String] { var current = "" }, "s")}")
    val c: Cell = new Cell { var v = 1 }
    c.v = 6; c.bump(); println(c.v)
    val e: Cell = new Cell { def v = st.cell; def v_=(n: Int) = st.cell = n + 100 }
    e.set(1); e.bump(); println(s"${e.v} ${st.cell}")
    val b = Box(1)
    Lib.assign(b, 10); (b: AbsVar).x += 1; println(b.x)
    val n = Counter(1)
    n.n = 4; n.n += 1; println(n.next)
    val o: Over = new OverCell
    o.w = 3; o.w += 1; o.w = "abcdefg"; println(o.w)
    val g: Handle[Int] = new GenCell
    g.current = 8; println(s"${g.current} ${GenCell().current(1)}")
