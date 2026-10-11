import fix.jargetter.*

@main def main(): Unit =
  val w: JgSocket = JgSocketImpl()
  println(w.send(1) + w.sendTwice(5) + w.send(7, true))
  println(JgLog.log)
