package csb

import csa.*

class D extends T

@main def run(): Unit =
  O.n_=(7)
  S.m_=(5)
  S.bump()
  val x = new C(2)
  val tt: T = x
  tt.t_=(10)
  x.c_=(20)
  csa.top_=(6)
  val f: Int => Unit = x.c_=
  f(30)
  val b = new Box("a")
  b.value_=("bb")
  O.n_=(3.toShort)
  val short = O.n
  O.n_=(8)
  val d = new D
  d.t_=(4)
  d.t = d.t + 1
  println(s"$short ${O.n} ${S.m} ${S.seen} ${x.t} ${x.c} ${csa.top} ${b.value} ${d.t}")
