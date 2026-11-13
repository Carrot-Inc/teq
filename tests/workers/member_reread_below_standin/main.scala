// jars: scala-library jdkparent-lib
// Eight units that each ask a library class for a member of the JDK that the std's stand-in two
// levels up leaves out: the library class's parent is a JDK class the std does not declare
// (`javax.management.AttributeList`), whose own parent is the std's `java.util.ArrayList`.
@main def run(): Unit =
  val x = new jdkparent.Listed
  println(List(A1.trim(x), A2.trim(x), A3.trim(x), A4.trim(x), A5.trim(x), A6.trim(x), A7.trim(x), A8.trim(x)).sum)
