// Assignment operators whose right side makes classes, typed where a kept answer of the worker's may or may not
// spare the first try (an earlier `x += 1` of the object: one worker meets it first, several may not): in a
// default argument and in a val with an inferred type; an anonymous class, one made in a lambda, a local class
// with a nested object (the JVM's lazy field), an assignment operator inside another's right side; a lazy val of
// an object nested in a local class, in a class of a local class, in an anonymous class of the operand (the
// object is owned by the class a dropped typing made and carries no mark of its own).
trait Sh { def g: Int }
object L {
  var x: Int = 1
  def first(): Unit = { x += 1 }
  def dflt(k: Int = { x += new Sh { def g = 2 }.g; 0 }): Int = k
  val inferred = { x += new Sh { def g = 3 }.g; x }
  def nested(k: Int = { x += { class K { object Ob { val v = 4 } }; new K().Ob.v }; 0 }): Int = k
  def lambda(k: Int = { x += ((i: Int) => new Sh { def g = i }.g)(5); 0 }): Int = k
  def inner(k: Int = { x += { var y = 1; y += new Sh { def g = 6 }.g; y }; 0 }): Int = k
  def nestedLazy(k: Int = { x += { class NK { object Ob { lazy val z = 1 } }; new NK().Ob.z }; 0 }): Int = k
  def deeperLazy(k: Int = { x += { class NK { class In { object Ob { lazy val z = 2 } } }; val n = new NK; new n.In().Ob.z }; 0 }): Int = k
  def anonLazy(k: Int = { x += new Sh { object Ob { lazy val z = 3 }; def g = Ob.z }.g; 0 }): Int = k
}
object Main {
  def main(args: Array[String]): Unit = {
    L.first()
    L.dflt()
    L.nested()
    L.lambda()
    L.inner()
    L.nestedLazy()
    L.deeperLazy()
    L.anonLazy()
    println(s"${L.x} ${L.inferred}")
  }
}
