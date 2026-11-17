// A lazy val of an object nested in a local class the right side of an assignment operator makes, in a default
// argument beside an earlier `x += 1`: the object is owned by a class a dropped first try made and carries no
// mark of its own; its JVM field has one number at every worker count and under the memo's check.
object L {
  var x: Int = 1
  def first(): Unit = { x += 1 }
  def dflt(k: Int = { x += { class NK { object Ob { lazy val z = 1 } }; new NK().Ob.z }; 0 }): Int = k
}
object Main { def main(args: Array[String]): Unit = { L.first(); L.dflt(); println(L.x) } }
