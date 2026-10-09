// After Scala 3's tests/neg/selfInheritance.scala: a class has to conform to the self type of
// the traits it mixes in.
// expect: illegal inheritance: self type Bad of class Bad does not conform to self type Logger & Named of parent trait Service
// expect: illegal inheritance: self type AlsoBad of class AlsoBad does not conform to self type Named of parent trait Other
// expect: illegal inheritance: self type Object with Service {...} of anonymous class Object with Service {...} does not conform to self type Logger & Named of parent trait Service
// expect: value name is not a member of Service
trait Logger:
  def log(s: String): String
trait Named:
  def name: String
trait Service:
  self: Logger & Named =>
  def run: String = log(s"running $name")
class Bad extends Service
class Good extends Service with Logger with Named:
  def log(s: String) = s"[$s]"
  def name = "good"
trait Other:
  this: Named =>
  def hi = name
class AlsoBad extends Other
object Main:
  val s = new Service {}
  val g: Service = new Good
  println(g.name)
