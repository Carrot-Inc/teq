// jars: scala-library jdkparent-lib
// Eight units that each ask a library class for a member it inherits from a JDK class the std
// does not declare (`Phaser.getRegisteredParties`), first at about the same moment.
@main def run(): Unit =
  val g = new jdkparent.Gate(2)
  println(List(U1.parties(g), U2.parties(g), U3.parties(g), U4.parties(g), U5.parties(g), U6.parties(g), U7.parties(g), U8.parties(g)).sum)
