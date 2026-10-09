// Predef's conversions from a box to its primitive give the primitive's zero for null, as
// scalac's `x.asInstanceOf[Int]` does, and the strict Boolean operators reach them for both
// operands.
@main def run(): Unit =
  val b: java.lang.Boolean = null
  val i: java.lang.Integer = null
  val l: java.lang.Long = null
  val d: java.lang.Double = null
  val f: java.lang.Float = null
  val c: java.lang.Character = null
  val s: java.lang.Short = null
  val y: java.lang.Byte = null
  println(true | b)
  println(false & b)
  println(b ^ true)
  println(true || b)
  val bb: Boolean = b
  println(bb)
  val ii: Int = i
  println(ii + 1)
  val ll: Long = l
  println(ll + 1L)
  val dd: Double = d
  println(dd + 1.5)
  val ff: Float = f
  println(ff + 1.5f)
  val cc: Char = c
  println(cc.toInt)
  val ss: Short = s
  println(ss + 1)
  val yy: Byte = y
  println(yy + 1)
  println(Predef.Integer2int(i) + Predef.Long2long(l))
  val full: java.lang.Integer = 41
  val t: java.lang.Boolean = true
  println(Predef.Integer2int(full) + 1)
  println(false | t)
  val boxes: List[java.lang.Integer] = List(1, null, 3)
  println(boxes.map(x => x + 0))
