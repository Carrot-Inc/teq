// The instance `isNaN` and `isInfinite` of `java.lang.Double` and `java.lang.Float`.
@main def run(): Unit =
  val values: List[java.lang.Double] = List(0.0 / 0.0, 1.0 / 0.0, 2.5).map(java.lang.Double.valueOf)
  println(values.map(d => s"${d.isNaN} ${d.isInfinite}").mkString(", "))
  val f: java.lang.Float = java.lang.Float.valueOf(Float.NaN)
  println(s"${f.isNaN()} ${f.isInfinite()}")
