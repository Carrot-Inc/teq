// What the entry point reaches stays; the rest (tests/dce.sh names it) is left out of the output.
package dce

trait Greeter:
  def greet(name: String): String = s"hello $name"
  def shout(name: String): String = greet(name).toUpperCase + "!"

case class Point(x: Int, y: Int):
  def norm1: Int = x.abs + y.abs
  def unusedScale(k: Int): Point = Point(x * k, y * k)

// Only a type test names it, so the class exists without a constructor call.
class Marker

class NeverMade:
  def whatever: Int = 1

object Used:
  val greeting: String =
    println("Used initialised")
    "hi"
  def helper(n: Int): Int = n + 1
  def unusedMethod(n: Int): Int = n - 1

object UnusedObj:
  val loud: String =
    println("UnusedObj initialised")
    "no"
  def talk(): Unit = println(loud)

enum Color:
  case Red, Green, Blue
  def isRed: Boolean = this == Red

// Reached through `values` alone, which names every case.
enum Dir:
  case North, South, East, West

def unusedFun(x: Int): Int = x * 2

def isMarker(x: Any): Boolean = x.isInstanceOf[Marker]

object Main extends Greeter:
  def main(args: Array[String]): Unit =
    println(greet("dce"))
    println(Point(3, -4).norm1)
    println(Used.greeting + Used.helper(1))
    println(Color.Red.isRed)
    println(Dir.values.length)
    println(isMarker(Point(1, 2)))
    println(usedVal)
