class C:
  override def toString = "C"
case class P(x: Int)

@main def run(): Unit =
  val c = new C
  val u: Unit = ()
  val i: Int = 1
  val s: String = "s"
  val o: Option[Int] = Some(1)
  val e: Int | String = 1
  val r: C | String = "r"
  println(s"${c} ${u} ${i} ${s} ${o} ${e} ${r} ${P(1)} ${'c'} ${1.5} ${null}")
  println(raw"${c}")
  println(f"${c}")
  println(f"${c}%s ${i}%d")
