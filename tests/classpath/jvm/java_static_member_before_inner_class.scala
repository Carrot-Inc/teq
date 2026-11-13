// jars: scala-library
// std: scala-library
// targets: jvm
// A class below a Java interface with a static member interface `Kind` and below a class with an inner class
// `Kind`, the Java interface first among its bases: a member typed `Kind` compiles and runs.
class Holder { class Kind(val n: Int) }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mine: Kind = null

@main def run(): Unit =
  println(new Ev().mine == null)
