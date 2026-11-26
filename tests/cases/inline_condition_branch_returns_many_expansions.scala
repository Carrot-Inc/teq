// The returning branch of inline_condition_branch_returns.scala expanded 3,277 times by nested inline methods into one
// method: scalac writes 65,032 bytes of code, within the class file's 65,535, so a jump written after each return
// would refuse a program scalac builds.
object Main {
  inline def pass(inline p: Boolean): Boolean = { println("pass"); p }
  inline def r(inline body: Unit): Unit = { body; body }
  inline def many(inline body: Unit): Unit = {
    r(r(r(r(r(r(r(r(r(r(r(body)))))))))))
    r(r(r(r(r(r(r(r(r(r(body))))))))))
    r(r(r(r(r(r(r(body)))))))
    r(r(r(r(r(r(body))))))
    r(r(r(body)))
    r(r(body))
    body
  }
  def work(x: Int): Boolean = {
    many { val y = if (x > 0) true && pass({ return true }) else false; () }
    false
  }
  def main(args: Array[String]): Unit = { println(work(1)); println(work(-1)) }
}
