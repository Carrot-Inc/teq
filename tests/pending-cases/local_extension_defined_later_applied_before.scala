// A local extension defined later in a method's block does not apply to a statement before it: on
// `super.apply(0) *= 3` scalac 3.8.4 takes the assignment form (`update`). teq applies the extension (master
// the same on `this.apply(0) *= 3` and on a plain method of such an extension).
class A1 { private var s: String = "a"; def apply(i: Int): String = { println("apply " + i); s }; def update(i: Int, e: String): Unit = { println("update " + i); s = e } }
class S extends A1 {
  def go(): Unit = {
    super.apply(0) *= 3
    extension (e: String) def *=(r: Int): Unit = { println("ext_later.*= " + e + " " + r) }
  }
}
object Main { def main(args: Array[String]): Unit = { val s = new S; s.go(); println(s(0)) } }
