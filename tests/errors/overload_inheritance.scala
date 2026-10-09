// expect: 12:7: error: class Partial needs to be abstract, since def visit(s: String): String in trait Visitor is not defined
// expect: 18:16: error: method run overrides nothing
// expect: 21:7: error: method run needs `override` modifier to override method run in trait Base
// expect: 29:47: error: type mismatch: found Boolean, required String
// expect: 30:23: error: None of the overloaded alternatives of method send in class Mailer with types
// expect:  (to: String, copies: Int): String
// expect:  (to: String): String
// expect: 5 errors found
trait Visitor:
  def visit(n: Int): String
  def visit(s: String): String
class Partial extends Visitor:
  def visit(n: Int): String = "int"

trait Base:
  def run(x: Int): Int = x
class Wrong extends Base:
  override def run(x: String): Int = 1
  override def run(x: Int): Int = 2
class Missing extends Base:
  def run(x: Int): Int = 2
  def run(x: Int, y: Int): Int = 3

trait Post:
  def send(to: String): String = to
class Mailer extends Post:
  def send(to: String, copies: Int): String = to * copies

@main def run(): Unit = println(Mailer().send(true))
def other(): String = Mailer().send()
