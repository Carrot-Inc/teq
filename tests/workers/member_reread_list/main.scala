// jars: scala-library
// Eight units that each meet `ArrayList.trimToSize` first at about the same moment: a member of
// the JDK's class that the std's `java.util.ArrayList` stands for and leaves out.
@main def run(): Unit =
  val xs = new java.util.ArrayList[String]()
  xs.add("a")
  println(List(L1.trim(xs), L2.trim(xs), L3.trim(xs), L4.trim(xs), L5.trim(xs), L6.trim(xs), L7.trim(xs), L8.trim(xs)).sum)
