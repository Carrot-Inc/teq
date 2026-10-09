//> using platform js
// The boxes of the numeric primitives are instances of java.lang.Number and answer its value
// methods through a Number-typed receiver, beside the classes extending it.
@main def run(): Unit =
  val xs: List[AnyRef] = List(Integer.valueOf(7), java.lang.Double.valueOf(1.5), java.lang.Long.valueOf(3L), java.lang.Short.valueOf(300.toShort), BigInt(2), BigDecimal(1), "s")
  for x <- xs do
    println(x match
      case n: java.lang.Number => s"number ${n.doubleValue} ${n.longValue} ${n.intValue} ${n.floatValue} ${n.byteValue} ${n.shortValue}"
      case _ => "other")
