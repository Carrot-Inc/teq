// The design pass's probe of the lowered interpreter's obligations (out/codex-probes/lowered-interpreter-design): a return through a finaliser, out of a closure, closures over the parameters of a tail-recursive def, a lazy local, a throwing finaliser replacing a return (its concatenation, which Scala.js renders otherwise, is tests/cases/lowered_concat.scala).
import scala.annotation.tailrec
object Main:
  def localReturn(): Int =
    try return 7 finally println("local-finally")
  @annotation.nowarn
  def closureReturn(): Int =
    try
      List(1).foreach { _ => return 8 }
      0
    finally println("closure-finally")
  @tailrec def loop(n: Int, fs: List[() => Int]): List[() => Int] =
    if n == 0 then fs else loop(n - 1, (() => n) :: fs)
  def capture(): () => Int =
    var x = 1
    lazy val y = { println("lazy"); x += 2; x }
    def get(): Int = y + x
    () => get()
  def main(args: Array[String]): Unit =
    println(localReturn())
    println(closureReturn())
    println(loop(3, Nil).map(_()).mkString(","))
    val f = capture()
    println(f()); println(f())
    try
      try return finally throw new IllegalArgumentException("finalizer")
    catch case e: IllegalArgumentException => println(e.getMessage)
