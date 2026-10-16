// jars: scala-library hiddenowner-lib
// teq: --target jvm --std=scala-library
// command: build
// expect: 20:21: error: Unable to emit reference to method method in class Base, class Base is not accessible in object Main
// expect: 21:21: error: Unable to emit reference to variable field in class Base, class Base is not accessible in object Main
// expect: 22:21: error: Unable to emit reference to method method in class Base, class Base is not accessible in object Main
// expect: 23:25: error: Unable to emit reference to variable field in class Base, class Base is not accessible in object Main
// expect: 24:23: error: Unable to emit reference to variable field in class Base, class Base is not accessible in object Main
// expect: 5 errors found
// A reference the JVM cannot link is refused at compile time, as dotty's `Erasure.typedSelect` refuses it
// (Erasure.scala 746-758, scalac 3.8.4's message): a public factory answers the package-private
// hidden.Inner, whose package-private superclass hidden.Base declares the member, so that neither the
// qualifier's class nor the declaring one can be named from another package (JVMS 5.4.4), and the
// qualifier's type before erasure is that class itself. A call, an inherited field read and written,
// and the qualifier naming the declaring class itself (hidden.Factory.base()). Through the public
// subclass the members are reached (tests/classpath/jvm/hidden_owner.scala).
package external

object Main:
  def call(): Int = hidden.Factory.make().method()
  def read(): Int = hidden.Factory.make().field
  def same(): Int = hidden.Factory.base().method()
  def sameRead(): Int = hidden.Factory.base().field
  def write(): Unit = hidden.Factory.make().field = 4
  def main(args: Array[String]): Unit = println(call())
