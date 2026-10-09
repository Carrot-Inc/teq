package nj

// Nested classes of the JDK, each read from a class file of its own.
class Use:
  def key(e: java.util.Map.Entry[String, Int]): String = e.getKey
  def state(t: Thread): Thread.State = t.getState
