// jars: scala-library hiddenowner-lib
// teq: --target jvm --std=scala-library
// command: build
// expect: 17:21: error: Unable to emit reference to method f in class Secret, class Secret is not accessible in object Main
// expect: 18:21: error: Unable to emit reference to variable value in class Secret, class Secret is not accessible in object Main
// expect: 2 errors found
// A Java class's accessibility is asked from here before erasure, as dotty's `Erasure.isJvmAccessible`
// asks it (`SymDenotations.isAccessibleFrom`), by the flags of a nested class's own `InnerClasses`
// entry: hidden.Nested's private static Secret is its enclosing class's alone, so that a call and a
// field read through it are refused even from its own package, with scalac 3.8.4's message. Its
// package's nested class and its protected one are named from here.
package hidden

object Main:
  def packaged(): Int = Nested.packaged().f()
  def guarded(): Int = Nested.guarded().f()
  def call(): Int = Nested.secret().f()
  def read(): Int = Nested.secret().value
  def main(args: Array[String]): Unit = println(List(packaged(), guarded(), call(), read()))
