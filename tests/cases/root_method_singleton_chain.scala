trait Marker
class Label extends Marker:
  override def toString: String = "label"

@main def run(): Unit =
  val a: Marker = new Label
  val b: a.type = a
  val c: b.type = b
  val d: c.type = c
  val e: d.type = d
  val f: e.type = e
  val g: f.type = f
  val h: g.type = g
  val i: h.type = h
  val j: i.type = i
  val k: j.type = j
  val l: k.type = k
  println(l.toString)
  println(s"value: $l")
