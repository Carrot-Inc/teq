//> using platform js
// `Throwable.setStackTrace`, and `getStackTrace` giving the frames set back.
@main def main(): Unit =
  val e = new RuntimeException("boom")
  e.setStackTrace(Array(new StackTraceElement("C", "m", "C.scala", 3), new StackTraceElement("D", "n", "D.scala", 4)))
  val frames = e.getStackTrace
  println(frames.length)
  println(frames(1).getMethodName)
  println(frames(0).getLineNumber)
  println(new RuntimeException("fresh").getStackTrace.length >= 0)
