// jars: scala-library
// std: scala-library
// targets: jvm
// A class and an object below `javax.swing.JButton` load: their class files list none of the interfaces
// the superclass brings, a package-private one of the JDK among them.
object Held extends javax.swing.JButton("o")
class Mine extends javax.swing.JButton("m")

@main def run(): Unit =
  System.setProperty("java.awt.headless", "true")
  println(Held.getText + " " + new Mine().getText)
