package fix.jargetter

// A jar's getter is a member of its pickle (`Desugar.addDefaultGetters`), which a body of the jar
// applies by name and an override with no default of its own calls through `this`
// (`Applications.findDefaultGetter`): the class has one getter of the name.
object JgLog:
  var log: String = ""
  def mark(s: String): String = { log = log + s + " "; s }

trait JgSocket:
  def send(x: Int, cont: Boolean = false, label: String = JgLog.mark("label")): String
  def sendTwice(x: Int): String = send(x) + send(x + 1, label = "two")

class JgSocketImpl extends JgSocket:
  def send(x: Int, cont: Boolean, label: String): String = s"$x$cont$label "
