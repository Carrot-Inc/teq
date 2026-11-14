// jars: scala-library javafix
// A Java inner class named through its outer class's companion: it is no member of the object, which holds
// the static member classes alone (scalac: `type Inner is not a member of object fix.Outer`).
// expect: type Inner not found
package fix
object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer[String]()
    val i = new Outer.Inner[Int]()
