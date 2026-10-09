// jars: scala-library
// std: scala-library
// Every primitive is an `AnyVal` (on JavaScript a boxed number has no width, so this runs on the JVM): `Byte`, `Short` and `Float` were refused as one where scalac
// admits all nine; a value of each boxed as an `AnyVal` keeps its class.
object Main:
  val b: Byte = 1
  val s: Short = 1
  val c: Char = 1
  val i: Int = 1
  val l: Long = 1
  val f: Float = 1
  val d: Double = 1
  val z: Boolean = true
  val u: Unit = ()
  val vb: AnyVal = b
  val vs: AnyVal = s
  val vc: AnyVal = c
  val vi: AnyVal = i
  val vl: AnyVal = l
  val vf: AnyVal = f
  val vd: AnyVal = d
  val vz: AnyVal = z
  val vu: AnyVal = u
  def main(args: Array[String]): Unit =
    println(List[AnyVal](vb, vs, vc, vi, vl, vf, vd, vz).map(_.getClass.getName).mkString(" "))
