// jars: scala-library javafix
// A Java inner class needs an outer instance, which the loader does not supply.
// expect: not supported yet: inner class fix.Outer.Inner needs an outer instance
package fix
object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer[String]()
    val i = new Outer.Inner[Int]()
