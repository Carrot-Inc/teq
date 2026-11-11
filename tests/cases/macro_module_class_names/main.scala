package q

@main def run(): Unit =
  val o = new p.Outer
  println(M.sym[p.Top.type])
  println(M.sym[o.E.type])
  println(M.sym[o.I.type])
