// jars: scala-library
// std: scala-library
// targets: jvm
// A member class of a Java class that is not static is inherited, as in Java: `LeafElement` of
// `AbstractDocument` inside a class below `PlainDocument`.
class Doc extends javax.swing.text.PlainDocument:
  def leaf: LeafElement = null
  def kind: String = "doc"

@main def run(): Unit =
  println(new Doc().kind + (new Doc().leaf == null))
