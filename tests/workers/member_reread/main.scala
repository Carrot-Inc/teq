// jars: scala-library
// Eight units that each meet `Class.getResource` first at about the same moment: a member of the
// JDK's class that the std's `java.lang.Class` stands for and leaves out.
@main def run(): Unit =
  println(List(R1.resource, R2.resource, R3.resource, R4.resource, R5.resource, R6.resource, R7.resource, R8.resource).count(_ == null))
