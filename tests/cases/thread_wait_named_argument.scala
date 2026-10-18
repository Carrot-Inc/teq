//> using platform jvm
// Object.wait's argument by name, the classfile parameter's x$0, as any application's arguments are
// matched (dotty's Applications.matchArgs).
object Main {
 def main(args: Array[String]): Unit = {
  val x = new Object
  x.synchronized { x.wait(x$0 = 1L) }
  println("ok")
 }
}
