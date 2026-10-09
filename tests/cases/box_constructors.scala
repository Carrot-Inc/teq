// The boxes' constructors, which the JDK deprecates but keeps: `new Integer(1)` is the box of 1 on
// every target (on JavaScript and in the interpreter the primitive itself, as Scala.js has it); and
// a primitive is an instance of its box, to a type test, a pattern and `Class.isInstance`.
object Main:
  def main(args: Array[String]): Unit =
    println(new java.lang.Integer(1).intValue())
    println(new java.lang.Integer("5") + 1)
    println(new java.lang.Long(2L))
    println(new java.lang.Short(3.toShort) == 3)
    println(new java.lang.Byte("7"))
    println(new java.lang.Boolean(true))
    println(new java.lang.Boolean("TRUE"))
    println(new java.lang.Double(1.5).doubleValue())
    println(new java.lang.Float(2.5f))
    println(new java.lang.Character('c'))
    val i: Integer = new Integer(3)
    println(i == 3)
    println((new Integer(4): Any).isInstanceOf[Integer])
    try new Integer("x") catch case e: NumberFormatException => println(e.getMessage)
    try new java.lang.Byte("300") catch case e: NumberFormatException => println(e.getMessage)
    val xs: List[Any] = List(1, 2L, 1.5, true, "s")
    println(xs.map(_.isInstanceOf[java.lang.Integer]))
    println(xs.map(_.isInstanceOf[java.lang.Long]))
    println(xs.map { case _: java.lang.Integer => "i"; case _: java.lang.Boolean => "z"; case _: java.lang.Double => "d"; case _ => "-" })
    println(s"${classOf[java.lang.Integer].isInstance(1)} ${classOf[java.lang.Long].isInstance(1)} ${classOf[java.lang.Boolean].isInstance(false)}")
