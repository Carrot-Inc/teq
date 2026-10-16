// jars: bridgetrait-lib
// A Java class whose parent is a Scala trait of the same jar (tests/support/hidden_owner/bridge), met
// through the Java class first: the trait is the package's member its TASTy defines, read by the TASTy
// loader as dotty's `SymbolLoaders.initializeFromClassPath` reads a class file beside its TASTy, not an
// empty Java class (teq reported `hello` a member of neither). Its member through the Java class is
// named on that class (`invokevirtual bridge/JavaReceiver.hello`), through the trait on the trait;
// java_scala_parent_trait_first.scala meets the trait first.
package javaparent

object Main:
  def throughJava(x: bridge.JavaReceiver): Int = x.hello()
  def throughTrait(x: bridge.Trait): Int = x.hello()
  def main(args: Array[String]): Unit =
    val x = new bridge.JavaReceiver()
    println(throughJava(x))
    println(throughTrait(x))
