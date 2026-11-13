// jars: scala-library
// std: scala-library
// targets: jvm
// A static member interface of a Java interface (`WatchEvent.Kind`) is not inherited: `Kind` inside a
// class that implements `WatchEvent` is the program's trait.
trait Kind

class Ev extends java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mine: Kind = new Kind {}

@main def run(): Unit =
  println(new Ev().mine != null)
