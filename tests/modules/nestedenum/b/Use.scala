package neb

@main def run(): Unit =
  val a = new nea.O(1)
  val b = new nea.O(10)
  println(a.E.A eq a.E.A)
  println(a.E.A eq b.E.A)
  println(s"${a.E.A.get} ${b.E.B.get} ${a.E.C(5).get} ${b.E.B.ordinal} ${a.E.A}")
  val p = new nea.P
  println(p.F.first eq p.F.X)
  println(p.F.X eq (new nea.P).F.X)
  val r = new nea.R
  println(r.G.Y eq r.G.Y)
  println(r.G.Y eq (new nea.R).G.Y)
  println(s"${p.F.values.toList} ${p.F.valueOf("X") eq p.F.X} ${p.F.fromOrdinal(0) eq p.F.X} ${r.G.values.length}")
