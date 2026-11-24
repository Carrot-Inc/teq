// jars: scala-library object-init-lib
// std: scala-library
// targets: jvm
// A class-path object that runs code when first loaded, as the left operand of a value class's right-associative
// extension called infix: scalac evaluates it before the right operand (an object with initialisers is no pure path,
// TreeInfo.refPurity), written through its package and imported. The program only references such an object, so its
// class is never completed and nothing records its initialisers: the JVM build takes it as one that runs code. A
// class-path object that runs nothing beside them. The objects are tests/support/object_init_lib.scala's.
import objectinit.{Log, LoudB, Quiet}

class P(val n: Int) extends AnyVal {
  extension (x: Any) def +::(y: String): String = y + x + n
}

object Main {
  def rd(): String = { Log.out = Log.out :+ "r"; "r" }
  def show(label: String, result: String): Unit = { println(label + " " + result + " " + Log.out.mkString(",")); Log.out = Nil }
  def main(args: Array[String]): Unit = {
    val v = new P(7)
    import v.*
    show("qualified", objectinit.LoudA +:: rd())
    show("imported", LoudB +:: rd())
    show("quiet", Quiet +:: rd())
  }
}
