object Probe { def f(): Long = try { throw new RuntimeException() } catch { case _: Exception => return 7L } finally { println("cleanup") }; def main(args: Array[String]): Unit = println(f()) }
