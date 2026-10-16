// jars: bridgetrait-lib
// java_scala_parent.scala with the Scala trait met before the Java class that implements it.
package javaparentfirst

object Main:
  def throughTrait(x: bridge.Trait): Int = x.hello()
  def throughJava(x: bridge.JavaReceiver): Int = x.hello()
  def main(args: Array[String]): Unit =
    val x = new bridge.JavaReceiver()
    println(throughTrait(x))
    println(throughJava(x))
