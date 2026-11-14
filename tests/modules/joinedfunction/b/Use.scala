package jfb

import jfa.*

trait Switched { def apply(b: Boolean): Int = 8 }
class Wider extends Scale with Switched

trait Extra { def apply(b: Boolean): Int = 5 }
abstract class Joined extends Chosen with Extra
class Named extends Chosen
class Mixed extends Chosen with Extra

object Use {
  def main(args: Array[String]): Unit = {
    val g: String => Int = new Length
    println(g("four"))
    val c = new Scale
    println((c: String => Int)("two"))
    println((c: Chosen).apply(None))
    val d = new Wider
    println((d: String => Int)("two"))
    println((d: Measure).apply("three"))
    println((d: Chosen).apply(None))
    println(d.apply(true))
    println(new Named().apply(None))
    val m = new Mixed
    println((m: Chosen).apply(Some(1)))
    println((m: Extra).apply(true))
  }
}
