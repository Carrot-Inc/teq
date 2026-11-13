// jars: scala-library jdkparent-lib
// std: scala-library
// targets: jvm
// A class and an object of the program below a library class below `javax.swing.JButton` load: their
// class files list none of the JDK ancestors' interfaces, a package-private one among them.
object Held extends jdkparent.Press("o")
class Mine extends jdkparent.Press("m")

@main def run(): Unit =
  System.setProperty("java.awt.headless", "true")
  println(Held.label + " " + new Mine().label)
