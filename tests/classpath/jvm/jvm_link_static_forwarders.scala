// jars: scala-library abi-callbacks-lib
// std: scala-library
//> using mainClass run
// Static forwarders as scalac writes them, called by reflection: an object's members in its
// mirror class (setters, lazy vals, members it overrides), a companion
// object's in its class, a case class companion's `apply`, top-level defs in `f$package`, and
// the `main(String[])` of an object that the `java` launcher runs.
import abi.Reflect

trait Greeting:
  def greet(name: String): String = s"hello $name"
object Registry extends Greeting:
  var count: Int = 1
  lazy val label: String = "registry"
  def scaled(n: Int, by: Int = 10): Int = n * by
  override def greet(name: String): String = s"hi $name"
class Account(val id: Int)
object Account:
  def open(id: Int): Account = Account(id)
  def describe(a: Account): String = s"account ${a.id}"
case class Item(name: String, qty: Int = 1)
object Tool:
  def main(args: Array[String]): Unit = println(s"Tool.main ${args.mkString(",")}")
def topLevel(x: Int): Int = x + 1

@main def run(): Unit =
  println(Reflect.static("Registry", "count"))
  Reflect.static("Registry", "count_$eq", 5)
  println(Registry.count)
  println(Reflect.static("Registry", "label"))
  println(Reflect.static("Registry", "scaled", 2, 3))
  println(Reflect.static("Registry", "greet", "you"))
  println(Reflect.static("Account", "describe", Account(7)))
  println(Reflect.static("Item", "apply", "pen", 3))
  println(Reflect.static("jvm_link_static_forwarders$package", "topLevel", 41))
  Reflect.runMain("Tool", "a", "b")
